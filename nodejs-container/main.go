package main

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"strings"

	"github.com/bytedance/sonic"
)

const (
	CONTAINER_NAME = "nodejs"
	DOCKER_IMAGE   = "nodejs:latest"
)

func main() {
	container := getContainer()
	if container == nil {
		fmt.Println("Failed to start docker container")
		os.Exit(1)
	}

	if !strings.HasPrefix(container.Status, "Up") {
		fmt.Printf("boot container %s, status was: %s\n", CONTAINER_NAME, container.Status)
		startContainer()
	}

	execContainer()
}

type Container struct {
	Command      string  `json:"Command"`
	CreatedAt    string  `json:"CreatedAt"`
	ID           string  `json:"ID"`
	Image        string  `json:"Image"`
	Labels       string  `json:"Labels"`
	LocalVolumes string  `json:"LocalVolumes"`
	Mounts       string  `json:"Mounts"`
	Names        string  `json:"Names"`
	Networks     string  `json:"Networks"`
	Platform     *string `json:"Platform"`
	Ports        string  `json:"Ports"`
	RunningFor   string  `json:"RunningFor"`
	Size         string  `json:"Size"`
	State        string  `json:"State"`
	Status       string  `json:"Status"`
}

func listContainers() []Container {
	args := []string{"ps", "-a", "--format=json"}

	cmd := exec.Command("docker", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		fmt.Println("Failed to list docker containers due to error:", err)
		os.Exit(1)
	}

	lines := bytes.Split(out, []byte("\n"))

	var containers []Container
	for _, line := range lines {
		// end of output
		if len(line) == 0 {
			continue
		}

		var container Container
		err = sonic.ConfigDefault.Unmarshal(line, &container)
		if err != nil {
			fmt.Printf("LINE: %+v\n", line)
			fmt.Println("Failed to parse docker container list:", err)
			os.Exit(1)
		}
		containers = append(containers, container)
	}

	return containers
}

var retry = 0

func getContainer() *Container {
	if retry > 3 {
		return nil
	}

	containers := listContainers()
	index := slices.IndexFunc(containers, func(c Container) bool { return c.Names == CONTAINER_NAME })

	if index == -1 {
		createContainer()

		if c := getContainer(); c != nil {
			return c
		}

		retry++

		return getContainer()
	}

	// reset retry counter
	retry = 0

	return &containers[index]
}

func createContainer() {
	home, _ := os.UserHomeDir()
	args := []string{
		"run", "-it",
		// run directly in the background
		"-d",
		"--name=" + CONTAINER_NAME,
		// volumnes to mount
		"-v", filepath.Join(home, ".ssh"), "/root/.ssh",
		"-v", filepath.Join(home, "workspace"), filepath.Join(home, ".ssh"),
		// docker image to use
		DOCKER_IMAGE,
	}

	cmd := exec.Command("docker", args...)
	out, err := cmd.CombinedOutput()
	if err == nil {
		return
	}

	fmt.Println("Failed to list docker containers due to error:", err)
	fmt.Println("FULL OUTPUT:", string(out))
	os.Exit(1)
}

func startContainer() {
	args := []string{
		"container", "start",
		CONTAINER_NAME,
	}

	cmd := exec.Command("docker", args...)
	cmd.Env = os.Environ()
	out, err := cmd.CombinedOutput()
	if err != nil {
		fmt.Println("Could not start docker container due to error:", err)
		fmt.Println("FULL OUTPUT:", string(out))
		os.Exit(1)
	}
}

func execContainer() {
	others := os.Args[1:]
	cwd, _ := os.Getwd()
	args := []string{
		"exec", "-it",
		"-w", cwd,
		// docker image to use
		CONTAINER_NAME,
		// enter bash
		"bash",
	}
	args = append(args, others...)

	cmd := exec.Command("docker", args...)
	cmd.Env = os.Environ()
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	err := cmd.Start()
	if err != nil {
		fmt.Println("Could not executing docker container due to error:", err)
		os.Exit(1)
	}
	cmd.Wait()
}

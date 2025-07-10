package main

import (
	"fmt"
	"os"
	"os/exec"
	"strconv"
	"strings"
)

const LINE_LENGTH = 100

func main() {
	input := os.Args[1]
	if len(input) == 0 {
		fmt.Println("Please input a payload e.g. [255, 220, ....]")
		fmt.Println("All bytes > 127 will be type cast as: (byte) 255")
		os.Exit(1)
	}

	input = strings.TrimSpace(input)
	input = strings.TrimPrefix(input, "[")
	input = strings.TrimSuffix(input, "]")

	items := strings.Split(input, ",")
	total := len(items)
	var out strings.Builder
	lineLength := 0
	for i, b := range items {
		// trim any spaces
		b = strings.TrimSpace(b)

		// convert into number
		c, err := strconv.Atoi(b)
		if err != nil {
			fmt.Printf("Cannot convert '%s', error: %s\n", b, err)
			os.Exit(1)
		}

		byt := b
		if c > 127 {
			byt = "(byte)" + b
		}

		// in case we have a next item
		if i < total && i != (total-1) {
			byt += ", "
		}

		if (lineLength + len(byt)) > LINE_LENGTH {
			out.WriteString("\n")
			lineLength = len(byt)
		}

		_, err = out.WriteString(byt)
		if err != nil {
			fmt.Printf("Cannot write '%s', error: %s\n", b, err)
			os.Exit(1)
		}
		lineLength += len(byt)
	}

	gen := out.String()

	copyClipboard(gen)

	fmt.Println("OUTPUT:")
	fmt.Println(gen)
	fmt.Println("")
	fmt.Println("Copied into clipboard!")
}

func copyClipboard(content string) error {
	cmd := exec.Command("pbcopy")
	in, err := cmd.StdinPipe()
	if err != nil {
		return err
	}
	if err := cmd.Start(); err != nil {
		return err
	}
	if _, err := in.Write([]byte(content)); err != nil {
		return err
	}
	if err := in.Close(); err != nil {
		return err
	}
	return cmd.Wait()
}

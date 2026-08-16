use std::{
    borrow::Borrow,
    env::{self, args},
    fs,
    io::Write,
    path::{self, PathBuf},
    process::{Command, Stdio, exit},
};

struct ProjectPath {
    path: PathBuf,
    basename: String,
}

impl ProjectPath {
    pub fn new(path: PathBuf) -> Self {
        let basename = path
            .clone()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        Self { path, basename }
    }

    pub fn path_to_string(&self) -> String {
        self.path.as_path().to_str().unwrap().to_string()
    }

    pub fn from_string(path: impl Into<String>) -> Self {
        Self::new(PathBuf::from(path.into()))
    }
}

impl Borrow<str> for ProjectPath {
    fn borrow(&self) -> &str {
        self.path.as_path().to_str().unwrap()
    }
}

fn main() {
    let args: Vec<String> = args().collect();

    let enabled_wtp = args.contains(&"--wtp".to_owned());
    let disabled_fzf = args.contains(&"--no-fzf".to_owned());
    let disabled_tmux = args.contains(&"--no-tmux".to_owned());

    let paths = parse_paths(&args, enabled_wtp);

    if disabled_fzf {
        println!("{}", paths.join("\n"));
        exit(0);
    }

    let selected = open_in_fzf(paths);
    if disabled_tmux {
        println!("{}", selected);
        exit(0);
    }

    open_in_tmux(ProjectPath::from_string(selected));
}

/// Paring the arguments and read the paths to check for worktrees with 'wtp'
fn parse_paths(args: &Vec<String>, enabled_wtp: bool) -> Vec<ProjectPath> {
    let mut paths: Vec<ProjectPath> = Vec::new();

    args.into_iter()
        .skip(1)
        .into_iter()
        .filter(|a| !a.starts_with("--"))
        .map(|p| path::absolute(p))
        .filter(|p| p.is_ok())
        .map(|p| p.unwrap())
        .for_each(|loc| {
            let entries = fs::read_dir(&loc)
                .expect(format!("Dir does not exists: {:?}", loc.as_os_str()).as_str());

            for f in entries {
                let mut p = f.as_ref().unwrap().path();
                if !p.is_dir() {
                    continue;
                }

                if enabled_wtp {
                    p.push(".wtp.yml");
                    if p.exists() {
                        p.pop();
                        request_wtp_list(&mut p, &mut paths);
                        continue;
                    }
                    p.pop();
                }

                paths.push(ProjectPath::new(p));
            }
        });

    paths
}

fn request_wtp_list(path: &mut PathBuf, paths: &mut Vec<ProjectPath>) {
    let c = Command::new("wtp")
        .arg("list")
        .current_dir(path.as_path())
        .output()
        .expect("Failed calling WTP command");

    path.push("worktrees");

    String::from_utf8(c.stdout)
        .unwrap()
        .lines()
        .skip(2)
        .map(|l| l.split_once(' ').unwrap().0)
        .for_each(|l| {
            let mut tree = path.clone();
            tree.push(l);
            paths.push(ProjectPath::new(tree));
        });
}

fn open_in_fzf(paths: Vec<ProjectPath>) -> String {
    let mut cmd = Command::new("fzf")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Command failed");

    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(paths.join("\n").as_bytes())
        .unwrap();
    let output = cmd.wait_with_output().unwrap();

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// After listing sessions check if there is already a session open.
/// otherwise we startup a new session.
fn open_in_tmux(project: ProjectPath) {
    let session = &project.basename;
    let session_path = &project.path_to_string();

    let in_tmux = env::var("TMUX").is_ok();

    if !in_tmux {
        let _ = Command::new("tmux")
            .args([
                "new-session",
                "-A",
                "-D",
                "-s",
                &project.path_to_string(),
                "-c",
                session_path,
            ])
            .status();
        return;
    }

    let status = Command::new("tmux")
        .args(["has-session", "-t", session])
        .status()
        .unwrap();

    // Already exists
    if status.success() {
        let _ = Command::new("tmux")
            .args(["switch-client", "-t", session])
            .output();
        return;
    }

    let _ = Command::new("tmux")
        .args(["new-session", "-D", "-s", session, "-c", session_path])
        .output();

    let _ = Command::new("tmux")
        .args(["switch-client", "-t", session])
        .output();
}

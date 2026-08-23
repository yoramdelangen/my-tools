use std::{
    borrow::Borrow,
    env::{self, args},
    fs,
    io::Write,
    path::{self, PathBuf},
    process::{Command, Stdio, exit},
};

mod config;

use config::{publish_config, read_config};

#[derive(Debug)]
struct ProjectPath {
    icon: Option<String>,
    path: PathBuf,
    session_name: String,
}

const DELIMITER: &str = " ";

impl ProjectPath {
    pub fn new(path: PathBuf) -> Self {
        let basename = Self::basename_from_pathbuf(path.clone());

        Self {
            path,
            session_name: basename,
            icon: None,
        }
    }

    pub fn basename_from_pathbuf(path: PathBuf) -> String {
        path.file_name().unwrap().to_string_lossy().to_string()
    }

    pub fn session_name(mut self, session_name: String) -> Self {
        self.session_name = session_name;
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
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

    if args.get(1).map(String::as_str) == Some("publish-config") {
        publish_config(args.contains(&"--overwrite".to_owned()));
        return;
    }

    let config = read_config();

    let enabled_wtp = config.wtp || args.contains(&"--wtp".to_owned());
    let enabled_git_worktree =
        config.git_worktree && !args.contains(&"--no-git-worktree".to_owned());
    let disabled_fzf = args.contains(&"--no-fzf".to_owned());
    let disabled_tmux = args.contains(&"--no-tmux".to_owned());

    let arg_paths: Vec<String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect();

    let paths = parse_paths(
        if arg_paths.is_empty() {
            &config.paths
        } else {
            &arg_paths
        },
        enabled_wtp,
        enabled_git_worktree,
    );

    if disabled_fzf {
        println!("{}", paths.join("\n"));
        exit(0);
    }

    let selected = open_in_fzf(paths);
    // when command has been canceld
    if selected.is_empty() {
        return;
    }

    let mut selected: Vec<&str> = selected.split(DELIMITER).collect();
    selected.reverse();

    let session_name = selected[1].to_owned();
    let path = selected[0].to_owned();

    if disabled_tmux {
        println!("PATH={} SESSION={}", path, session_name);
        exit(0);
    }

    open_in_tmux(ProjectPath::from_string(path).session_name(session_name));
}

/// Parse the paths and check for worktrees with 'wtp'.
fn parse_paths(args: &[String], enabled_wtp: bool, enabled_git_worktree: bool) -> Vec<ProjectPath> {
    let mut paths: Vec<ProjectPath> = Vec::new();

    args.into_iter()
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

                if enabled_git_worktree {
                    p.push(".git");
                    let has_git = p.exists();
                    p.pop();
                    p.push(".bare");
                    let has_bare = p.exists();
                    p.pop();
                    if has_git || has_bare {
                        if request_git_worktree_list(&p, &mut paths) {
                            continue;
                        }
                    }
                }

                paths.push(ProjectPath::new(p));
            }
        });

    paths
}

fn request_git_worktree_list(path: &PathBuf, paths: &mut Vec<ProjectPath>) -> bool {
    let c = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(path.as_path())
        .output()
        .expect("Failed calling git worktree list");

    if !c.status.success() {
        return false;
    }

    let worktrees = parse_git_worktree_list(&String::from_utf8_lossy(&c.stdout));
    let found = !worktrees.is_empty();
    worktrees
        .into_iter()
        .for_each(|p| paths.push(ProjectPath::from_string(p).icon("🌳")));
    found
}

fn parse_git_worktree_list(output: &str) -> Vec<String> {
    let entries: Vec<(&str, bool)> = output
        .lines()
        .filter_map(|l| {
            l.split_once(' ')
                .map(|(path, _)| (path, l.contains("(bare)")))
        })
        .collect();

    if matches!(entries.as_slice(), [(_, false)]) {
        return Vec::new();
    }

    entries
        .into_iter()
        .map(|(path, bare)| {
            if bare {
                PathBuf::from(path)
                    .parent()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(path))
                    .to_string_lossy()
                    .to_string()
            } else {
                path.to_string()
            }
        })
        .collect()
}

fn request_wtp_list(path: &mut PathBuf, paths: &mut Vec<ProjectPath>) {
    let c = Command::new("wtp")
        .arg("list")
        .current_dir(path.as_path())
        .output()
        .expect("Failed calling WTP command");

    let basename = ProjectPath::basename_from_pathbuf(path.clone());

    parse_wtp_list(&String::from_utf8_lossy(&c.stdout), path)
        .into_iter()
        .for_each(|tree| {
            paths.push(
                ProjectPath::new(tree)
                    .session_name(basename.clone())
                    .icon("🌳"),
            );
        });
}

fn parse_wtp_list(output: &str, root: &PathBuf) -> Vec<PathBuf> {
    output
        .lines()
        .skip(2)
        .filter_map(|l| l.split_once(' ').map(|(name, _)| name.replace('@', "")))
        .map(|name| {
            if name.is_empty() {
                root.clone()
            } else {
                root.join("worktrees").join(name)
            }
        })
        .collect()
}

fn open_in_fzf(paths: Vec<ProjectPath>) -> String {
    let mut cmd = Command::new("fzf")
        .args([
            format!("{}{}", "--delimiter=", DELIMITER).as_str(),
            "--with-nth=1,3",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Command failed");

    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(
            paths
                .iter()
                .map(|p| {
                    [
                        p.icon.clone().unwrap_or("\u{00A0}\u{00A0}".to_string()),
                        p.session_name.clone(),
                        p.path_to_string(),
                    ]
                    .join(DELIMITER)
                    .to_string()
                })
                .collect::<Vec<_>>()
                .join("\n")
                .as_bytes(),
        )
        .unwrap();
    let output = cmd.wait_with_output().unwrap();

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// After listing sessions check if there is already a session open.
/// otherwise we startup a new session.
fn open_in_tmux(project: ProjectPath) {
    let session = &project.session_name;
    let session_path = &project.path_to_string();

    let in_tmux = env::var_os("TMUX").is_some();

    let status = Command::new("tmux")
        .args(["has-session", "-t", session])
        .status()
        .unwrap();

    println!("status {:?}", status);

    if !status.success() {
        let _ = Command::new("tmux")
            .args(["new-session", "-d", "-s", session, "-c", session_path])
            .status();
    }

    if in_tmux {
        let _ = Command::new("tmux")
            .args(["switch-client", "-t", session])
            .status();
        return;
    }

    let _ = Command::new("tmux")
        .args(["attach-session", "-t", session])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_git_worktree_list() {
        assert_eq!(
            parse_git_worktree_list(
                "/tmp/repo       abc1234 [main]\n/tmp/repo-foo   def5678 [foo]\n"
            ),
            ["/tmp/repo", "/tmp/repo-foo"]
        );
    }

    #[test]
    fn ignores_single_plain_git_worktree_entry() {
        assert!(parse_git_worktree_list("/tmp/repo       abc1234 [main]\n").is_empty());
    }

    #[test]
    fn parses_bare_git_worktree_as_project_root() {
        assert_eq!(
            parse_git_worktree_list(
                "/tmp/repo/.bare                  (bare)\n/tmp/repo/worktrees/foo   abc1234 [foo]\n"
            ),
            ["/tmp/repo", "/tmp/repo/worktrees/foo"]
        );
        assert_eq!(
            parse_git_worktree_list("/tmp/repo/.bare                  (bare)\n"),
            ["/tmp/repo"]
        );
    }

    #[test]
    fn parses_wtp_at_as_project_root() {
        assert_eq!(
            parse_wtp_list(
                "header\nheader\n@ active\n@foo active\n",
                &PathBuf::from("/tmp/repo")
            ),
            [
                PathBuf::from("/tmp/repo"),
                PathBuf::from("/tmp/repo/worktrees/foo")
            ]
        );
    }
}

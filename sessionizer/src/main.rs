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

#[derive(Default)]
struct Config {
    paths: Vec<String>,
    wtp: bool,
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
    let config = read_config();

    let enabled_wtp = config.wtp || args.contains(&"--wtp".to_owned());
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
    );

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

fn read_config() -> Config {
    #[cfg(debug_assertions)]
    let path = PathBuf::from("config.toml");

    #[cfg(not(debug_assertions))]
    let path = {
        let Some(home) = env::var_os("HOME") else {
            return Config::default();
        };
        PathBuf::from(home).join(".config/sessionizer/config.toml")
    };

    match fs::read_to_string(&path) {
        Ok(config) => parse_config(&config).unwrap_or_else(|e| panic!("{}: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => panic!("{}: {}", path.display(), e),
    }
}

fn parse_config(config: &str) -> Result<Config, String> {
    let mut parsed = Config::default();
    let mut lines = config.lines();

    while let Some(line) = lines.next() {
        let line = strip_comment(line).trim().to_string();
        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("invalid line: {line}"));
        };
        let key = key.trim();
        let mut value = value.trim().to_string();

        if key == "paths" {
            while !value.contains(']') {
                let Some(next) = lines.next() else {
                    return Err("unterminated paths array".to_string());
                };
                value.push('\n');
                value.push_str(strip_comment(next).trim());
            }
            parsed.paths = parse_string_array(&value)?;
        } else if key == "wtp" {
            parsed.wtp = match value.as_str() {
                "true" => true,
                "false" => false,
                _ => return Err("wtp must be true or false".to_string()),
            };
        }
    }

    Ok(parsed)
}

fn strip_comment(line: &str) -> String {
    let mut quoted = false;

    for (i, c) in line.char_indices() {
        if c == '"' {
            quoted = !quoted;
        } else if c == '#' && !quoted {
            return line[..i].to_string();
        }
    }

    line.to_string()
}

fn parse_string_array(value: &str) -> Result<Vec<String>, String> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err("paths must be an array of strings".to_string());
    }

    let mut paths = Vec::new();
    let mut quoted = false;
    let mut current = String::new();

    for c in value[1..value.len() - 1].chars() {
        if c == '"' {
            if quoted {
                paths.push(expand_home(&current));
                current.clear();
            }
            quoted = !quoted;
        } else if quoted {
            current.push(c);
        } else if !c.is_whitespace() && c != ',' {
            return Err("paths must contain only strings".to_string());
        }
    }

    if quoted {
        return Err("unterminated string in paths".to_string());
    }

    Ok(paths)
}

fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }

    path.to_string()
}

/// Parse the paths and check for worktrees with 'wtp'.
fn parse_paths(args: &[String], enabled_wtp: bool) -> Vec<ProjectPath> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let config = parse_config(
            r#"
                wtp = true
                paths = [
                    "/tmp/projects", # comment
                    "/tmp/other#project",
                ]
            "#,
        )
        .unwrap();

        assert!(config.wtp);
        assert_eq!(config.paths, ["/tmp/projects", "/tmp/other#project"]);
    }
}

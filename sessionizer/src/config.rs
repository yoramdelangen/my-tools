use std::{env, fs, path::PathBuf, process::exit};

pub(crate) struct Config {
    pub(crate) paths: Vec<String>,
    pub(crate) wtp: bool,
    pub(crate) git_worktree: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            wtp: false,
            git_worktree: true,
        }
    }
}

pub(crate) fn read_config() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };

    match fs::read_to_string(&path) {
        Ok(config) => parse_config(&config).unwrap_or_else(|e| panic!("{}: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => panic!("{}: {}", path.display(), e),
    }
}

fn config_path() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    return Some(PathBuf::from("config.toml"));

    #[cfg(not(debug_assertions))]
    {
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/sessionizer/config.toml"))
    }
}

pub(crate) fn publish_config(overwrite: bool) {
    #[cfg(debug_assertions)]
    {
        let _ = overwrite;
        eprintln!("publish-config is only available in production builds");
        exit(1);
    }

    #[cfg(not(debug_assertions))]
    {
        let Some(path) = config_path() else {
            panic!("HOME is not set");
        };
        if path.exists() && !overwrite {
            eprintln!("{} already exists", path.display());
            exit(1);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{}: {}", parent.display(), e));
        }

        fs::write(&path, default_config()).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
        println!("created {}", path.display());
    }
}

fn default_config() -> String {
    let config = Config::default();

    format!(
        "wtp = {}\ngit_worktree = {}\npaths = []\n",
        config.wtp, config.git_worktree
    )
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
        } else if key == "git_worktree" {
            parsed.git_worktree = match value.as_str() {
                "true" => true,
                "false" => false,
                _ => return Err("git_worktree must be true or false".to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let config = parse_config(
            r#"
                wtp = true
                git_worktree = true
                paths = [
                    "/tmp/projects", # comment
                    "/tmp/other#project",
                ]
            "#,
        )
        .unwrap();

        assert!(config.wtp);
        assert!(config.git_worktree);
        assert_eq!(config.paths, ["/tmp/projects", "/tmp/other#project"]);
    }

    #[test]
    fn defaults_git_worktree_on() {
        assert!(Config::default().git_worktree);
    }

    #[test]
    fn published_config_matches_defaults() {
        let config = parse_config(&default_config()).unwrap();

        assert!(!config.wtp);
        assert!(config.git_worktree);
        assert!(config.paths.is_empty());
    }
}

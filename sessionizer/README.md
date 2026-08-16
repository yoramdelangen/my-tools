# Sessionizer

Rust CLI for selecting a project with `fzf` and opening it in `tmux`.

It scans one or more directories, lists their direct child directories, lets you choose one with `fzf`, then switches to or creates a matching `tmux` session.

## Requirements

- Rust
- `fzf`
- `tmux`
- Optional: `wtp` for worktree project support

## Usage

```sh
cargo run -- ~/workspace/repositories ~/workspace/other-projects
```

Or put defaults in `~/.config/sessionizer/config.toml`.

## Example config

```toml
# ~/.config/sessionizer/config.toml
wtp = true
paths = [
  "~/workspace/repositories",
  "~/workspace/other-projects",
]
```

CLI paths override configured paths when provided.
Debug builds read `./config.toml` instead.

Useful flags:

- `--wtp`: if a project contains `.wtp.yml`, list its `wtp` worktrees instead of the project directory.
- `--no-fzf`: print discovered project paths and exit.
- `--no-tmux`: print the selected project path instead of opening `tmux`.

## Install

```sh
./build.sh
```

This builds the release binary and moves it to `~/.bin/rust-sessionizer`.

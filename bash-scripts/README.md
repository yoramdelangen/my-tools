# Bash Scripts

Small shell scripts for one-off automation tasks.

## Scripts

### `recursive-zipping.sh`

Creates a `.zip` file for every subdirectory under `recusive-zipping/`. Each archive is written next to the folder it zips and is named after that folder.

## Requirements

- Bash
- `zip`

## Usage

```sh
./recursive-zipping.sh
```

The script currently expects to be run from this directory and uses the hardcoded `recusive-zipping/` folder.

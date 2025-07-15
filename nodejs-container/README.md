# NodeJS Container

I prefer NodeJS stuff to be running from a Docker container instead of being installed on my machine.
Therefore this tool does create a proper container.

## Build

Building and distribution of the binary will be done via a `Makefile`.

```bash
make
```

> This will symlink the binary from the current folder into `$HOME/.bin/`.

## Usage

```bash
nodejs
```

## Missing

Currently there are only a few things we need todo manually when creating new container.

```bash
# enable yarn v4 instead of v1.xx
corepack enable
```

## TODO

The following things we should add as well:

- Allow for certain environment variables to be passed.
    - Missing: `TOKEN_FOR_GITHUB`
- Hydrayte `os.Environ()` so its not sending everything, including current OS only stuff.

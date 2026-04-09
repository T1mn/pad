# Platform Support

PAD (`pad`) is tested for:

- macOS
- Linux
- WSL2

## Runtime requirements

- `tmux` is required at runtime.
- `pad` and `tmux` must run in the same environment.
- On WSL2, install and run both `pad` and `tmux` inside WSL.
- `install.sh` can prompt to install `tmux` automatically on supported package managers.

## Supported release targets

- Linux x86_64 (glibc 2.35)
- Linux aarch64 (glibc 2.35)
- Linux x86_64 (musl)
- Linux aarch64 (musl)
- macOS universal

## Current non-goals

- Windows native support
- Mixed host setups such as Windows-host tmux with WSL `pad`

## Release validation

- CI: format, clippy, tests, dist build
- tmux smoke: macOS and Linux
- Manual smoke: WSL2 before release

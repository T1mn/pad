# Platform Support

`pad` is tested for:

- macOS
- Linux
- WSL2

## Runtime requirements

- `tmux` is required at runtime.
- `pad` and `tmux` must run in the same environment.
- On WSL2, install and run both `pad` and `tmux` inside WSL.

## Supported release targets

- Linux x86_64
- Linux aarch64
- macOS universal

## Current non-goals

- Windows native support
- Mixed host setups such as Windows-host tmux with WSL `pad`

## Release validation

- CI: format, clippy, tests, dist build
- tmux smoke: macOS and Linux
- Manual smoke: WSL2 before release

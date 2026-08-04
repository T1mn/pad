# Platform Support

PAD (`pad`) is tested for:

- macOS
- Linux
- WSL2

## Runtime requirements

- Native mode is the default and does not require `tmux`.
- `tmux` is required only for the explicit `pad --tmux` compatibility mode and legacy tmux-only session workflows.
- On WSL2, run `pad` inside WSL; when using compatibility mode, install and run `tmux` inside the same WSL environment.
- `install.sh` may offer `tmux` for compatibility workflows, but the native terminal can start without it.
- Set `PAD_HOME` to isolate PAD's config, runtime sockets, logs, and state from the default `~/.pad` directory.

## Supported release targets

- Linux x86_64 (glibc 2.35)
- Linux aarch64 (glibc 2.35)
- Linux x86_64 (musl)
- Linux aarch64 (musl)
- macOS universal

## Current non-goals

- Windows native support
- Mixed compatibility setups such as Windows-host tmux with WSL `pad`

## Release validation

- CI: format, clippy, tests, dist build
- native no-tmux smoke: macOS and Linux
- explicit tmux compatibility smoke: macOS and Linux
- Manual smoke: WSL2 before release

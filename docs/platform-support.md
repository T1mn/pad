# Platform Support

PAD (`pad`) is tested for:

- macOS
- Linux
- WSL2

## Runtime requirements

- PAD owns the shell PTY, terminal grid, tabs, splits, and child-process lifecycle.
- No external terminal multiplexer is required.
- On WSL2, run PAD and the agent CLIs inside the same WSL environment.
- Set `PAD_HOME` to isolate PAD's config, runtime sockets, logs, and state from the default `~/.pad` directory.

## Supported release targets

- Linux x86_64 (glibc 2.35)
- Linux aarch64 (glibc 2.35)
- Linux x86_64 (musl)
- Linux aarch64 (musl)
- macOS universal

## Current non-goals

- Windows native support
- Starting PAD on the Windows host while agent CLIs run inside WSL
- Preserving live child processes across a PAD restart
- Multi-client sharing of one live terminal workspace

## Release validation

- CI: format, clippy, tests, dist build
- native PTY interaction smoke: macOS and Linux
- Manual smoke: WSL2 before release

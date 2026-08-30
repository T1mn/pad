# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions are patch-driven;
the version and hotfix policy lives in `AGENTS.md`, and the release steps in
`docs/release-checklist.md`.

Entries were reconstructed from the commit history, so releases before 0.7.0 are summarized rather
than listed one by one.

## [Unreleased]

### Changed

- Replaced the Desktop Rust sidecar with an in-process TypeScript backend for SQLite, Pi RPC,
  authentication, model discovery, Fast mode, Full Access, terminal control, and remote access.
- Removed the legacy Rust TUI, SwiftUI fallback shell, sidecar compatibility client, CLI installer,
  and the CI/test suites that only protected those retired implementations.
- Reduced the repository to the macOS Desktop product and its native iOS remote companion.

### Fixed

- The packaged app now reads the authenticated Pi model catalog directly and inherits the effective
  macOS system proxy before starting Pi.
- The TypeScript remote gateway now speaks the existing `pad.remote.v1` WSS pairing and command
  protocol used by the iPhone app.

## [0.7.6] - 2026-08-30

### Added

- A native SwiftUI `PAD Remote` companion for iOS 17+, with Chinese task sidebar, live Pi
  conversation timeline, Composer controls, offline cache, durable command outbox, and QR pairing.
- A same-LAN WSS gateway in PAD's existing Desktop runtime, with TLS certificate pinning, bounded
  per-device queues, revision replay, epoch-based resynchronization, heartbeats, and UUID command
  receipts.
- PAD Desktop remote settings for starting the gateway, creating a 120-second one-time pairing QR,
  showing live device status, and revoking paired devices.

### Security

- Remote devices are bound to one PAD Profile and receive redacted DTOs through an explicit action
  allowlist; login, account switching, Full Access changes, terminals, credentials, and host paths
  remain unavailable remotely.
- Pairing secrets are single-use, device tokens are stored as hashes on the Mac and in the iOS
  Keychain, private gateway files use restrictive permissions, and revocation terminates the active
  connection as well as future resumes.
- Remote cache, revision cursor, and outbox state are isolated by paired host identity so one Mac or
  Profile cannot inherit another pairing's tasks or queued commands.

### Fixed

- Pi control responses no longer enter the task reducer as failures, and same-poll interaction
  events are deferred to the single Desktop poll owner instead of being swallowed by request paths.
- Streaming `message_start` / `text_delta` / `message_end` events now update one stable assistant
  message and reconcile with authoritative history without duplicate output.
- Duplicate command IDs are tied to the original action and payload; conflicting reuse fails closed,
  while retransmission of the same mutation returns the persisted receipt without executing twice.
- iOS transport callbacks now cross one ordered mailbox, and every outbox operation remains bound to
  its connection generation and paired-host identity across actor suspension.
- The aggregate runtime-status test now enters its cross-process lock-holder fast path before the
  expanded Desktop/Remote compact suite, eliminating a deterministic five-second false timeout.

## [0.7.5] - 2026-08-29

### Added

- A directly installable Apple Silicon `PAD Desktop.app` built with Electron and React, with a
  tiled macOS Codex-style hierarchy, complete Simplified Chinese UI, typed preload boundary, task
  timeline, Composer, settings, inspector, and a real NativePty bottom terminal.
- Visual Pi account creation, provider login, logout, isolated Profile switching, persistent UI
  state, and account-scoped task history backed by PAD SQLite.
- Packaged PAD, Bun, and Pi runtimes plus deterministic ZIP/DMG release, visual-matrix, black-box
  isolation, accessibility, and performance acceptance scripts.

### Security

- Desktop protocol v2 exposes renderer-safe DTOs, strict action fields and 1 MiB frames; private
  credentials, environment values, Pi journals, and PAD/Codex/ChatGPT paths are redacted.
- A single-writer data-root lock, per-Profile Pi roots, and Full Access policy keep ordinary work
  unattended while continuing to deny PAD/Pi/Codex/ChatGPT private data, macOS TCC, credentials,
  and cross-Profile access.

## [0.7.4] - 2026-08-03

### Security

- Relay and provider configuration is written through an atomic, permission-checked path instead of
  being rewritten in place.
- The local control socket now authenticates its peer before accepting requests.
- Runtime status reconciles process identity before acting on a recorded PID, so a reused PID is not
  mistaken for a tracked agent.
- Remote (`ssh`) launch commands validate and quote their arguments instead of building a shell line
  from raw host and command strings.
- The installer verifies checksums for downloaded release artifacts.

### Added

- `SECURITY.md` with a private reporting channel and response targets.
- `CONTRIBUTING.md` documenting the repository's CI-enforced structural rules.
- This changelog.
- GitHub issue forms, a pull request template, and a Dependabot config for cargo and
  github-actions.
- A `cargo audit` job in CI that scans `rust-tui` dependencies against the RustSec database.
- `rust-version` (MSRV) in `rust-tui/Cargo.toml`.

### Fixed

- Relay: skip rewriting live provider config on startup.
- Relay settings: a non-ASCII API key or Codex auth token no longer crashes the detail pane. The
  mask truncated by byte offset while guarding by byte length, so a multi-byte character could be
  split. Because the key is persisted, this crashed on every render of the pane.
- Codex archive: a rollout filename with multi-byte characters is rejected instead of panicking.
  The date segments were sliced before the filename was validated, and checking only the `-`
  separators still allowed a split character in the day segment.
- Panel path shortening no longer underflows when the available width is under 3 columns.
- The fuzzy picker no longer underflows when the terminal is too short to give it any height.

## [0.7.3] - 2026-07-17

### Added

- Grok Build support: session discovery, history in the sidebar, and transcript preview.
- `docs/agent-compatibility.md` matrix plus `docs/grok-support.md`, and an updated OpenCode support
  page.
- Wider OpenCode diagnostics report and stats export.
- DeepSeek launcher path in the relay.
- Codex preview handles compressed transcript segments.

### Changed

- Archive is documented and implemented per agent: Codex moves the rollout between its active and
  archive directories and updates its state DB, OpenCode updates `time_archived`, Claude uses PAD's
  local index. None of them deletes the upstream conversation.

### Fixed

- tmux: pane handoff and the return binding now target a writable client explicitly, instead of
  assuming the current one.
- CI: the tmux smoke test drives a writable client, which unblocked the macOS runs.

## [0.7.2] - 2026-06-12

### Added

- DeepSeek (cc) relay provider, with `docs/deepseek-setup.md`, `docs/deepseek-env-only.md`, and
  helper scripts under `scripts/`.
- Adjustable agent list width, bound to `L` and persisted in the display config.
- Safety-focused relay provider config tests for Claude and OpenCode.

### Changed

- Panel list: thread subtitle rendering folded back into the thread row, and panel width
  calculation reworked around the stored preference.

## [0.7.1] - 2026-06-09

### Added

- Relay settings run real chat provider probes instead of static configuration checks.

## [0.7.0] - 2026-06-09

### Added

- Claude relay provider support.
- CI checks for the repository structure: `scripts/ci/check_index.py` for per-directory `index.md`
  files, and `scripts/ci/check_rust_file_size.py` for the 200-line cap and the ban on inline test
  modules.

### Changed

- Large modules split so every file under `rust-tui/src` fits the 200-line cap, and all unit tests
  moved out of source files into external `*_tests.rs` files.
- Sustained allocation-reduction pass across the sidebar, pad-sider, fuzzy picker, session cache,
  Codex diff, telegram daemon, and tmux scanner paths; dead helpers and unused facade exports
  removed.

## [0.6.59] and earlier - 2026-03-30 - 2026-06-08

The 0.6.x line built the product itself: the tmux-first workspace and sidebar, agent state
detection, session history and preview for Codex, Claude Code, Gemini and OpenCode, the `F10`
pad-sider (file tree, project index map, split preview, line numbers, Codex turn diff review),
multi-provider relay and proxy settings, the Telegram bot daemon, completion notifications, six-language
i18n, the native fuzzy picker, agent workflow automation, and the installer plus release pipeline.

[Unreleased]: https://github.com/T1mn/pad/compare/v0.7.6...HEAD
[0.7.6]: https://github.com/T1mn/pad/compare/v0.7.5...v0.7.6
[0.7.5]: https://github.com/T1mn/pad/compare/v0.7.4...v0.7.5
[0.7.4]: https://github.com/T1mn/pad/compare/v0.7.3...v0.7.4
[0.7.3]: https://github.com/T1mn/pad/compare/v0.7.2...v0.7.3
[0.7.2]: https://github.com/T1mn/pad/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/T1mn/pad/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/T1mn/pad/compare/v0.6.59...v0.7.0
[0.6.59]: https://github.com/T1mn/pad/releases/tag/v0.6.59

# Release Checklist

## Pre-release

1. Ensure the worktree is in the expected state.
2. Update `rust-tui/Cargo.toml` version.
3. Review release notes and user-visible changes.
4. Run `bash scripts/build_installer.sh` and confirm `install.sh` is up to date.
5. Confirm `install.sh` still matches the published artifacts.

## Local verification

1. Run `cargo fmt --check` in `rust-tui/`.
2. Run `cargo clippy --all-targets --all-features -- -D warnings` in `rust-tui/`.
3. Run `cargo test` in `rust-tui/`.
4. Run `cargo build --profile dist` in `rust-tui/`.
5. Run `bash scripts/build_installer.sh`.
6. Launch `pad` locally at least once.
7. Run `PAD_INSTALL_FORCE_SOURCE=1 PAD_INSTALL_ASSUME_YES=1 INSTALL_DIR="$(mktemp -d)" ./install.sh`.

## Automated checks

1. Confirm `CI` workflow is green.
2. Confirm `Tmux Smoke` workflow is green on macOS and Linux.
3. Confirm the release tag matches `rust-tui/Cargo.toml`.
4. Confirm the installer smoke job is green.

## Manual smoke before publishing

1. Run inside WSL2, not on the Windows host shell.
2. Verify `tmux` is installed in WSL2.
3. Start at least one real AI agent inside tmux.
4. Launch `pad`.
5. Verify scan, preview, attach, and detach.

## Publish

1. Create and push a tag like `v0.6.0`.
2. Wait for the `Release` workflow to finish.
3. Download one release artifact and verify `pad --help`.
4. Verify `curl -fsSL https://raw.githubusercontent.com/T1mn/pad/main/install.sh | bash` still works.

## Rollback

1. If release artifacts are broken, delete the GitHub Release.
2. Delete the bad tag locally and remotely.
3. Fix the issue, retag, and rerun the release workflow.

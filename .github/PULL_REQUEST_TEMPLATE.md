# Summary

<!-- What changes and why. Link the issue if there is one. -->

## Checklist

Structural rules, all enforced by CI (details in `CONTRIBUTING.md`):

- [ ] Every touched `rust-tui/src/**/*.rs` file is at most 200 lines (`rust-tui/src/i18n/` is exempt).
- [ ] Cohesive implementation and tests stay discoverable together; avoid recreating tiny leaf modules.
- [ ] `python3 scripts/ci/check_rust_file_size.py` passes.
- [ ] Every new directory has an `index.md` of at most 50 lines, and the `index.md` files touched by
      this change still point at paths that exist.
- [ ] `python3 scripts/ci/check_index.py` passes.
- [ ] If `install/` changed: ran `bash scripts/build_installer.sh` and committed the regenerated
      `install.sh` (`git diff --exit-code -- install.sh` is clean).

Local verification:

- [ ] `cd rust-tui && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test`

Change hygiene:

- [ ] Commit messages follow Conventional Commits (`feat(scope): ...`, `fix(scope): ...`).
- [ ] Behavior changes come with tests.
- [ ] User-visible changes are noted under `[Unreleased]` in `CHANGELOG.md`.
- [ ] No version bump in this PR (releases are maintainer-driven, see `docs/release-checklist.md`).

## Verification

<!-- What you ran, on which platform and shell, and what you saw. -->

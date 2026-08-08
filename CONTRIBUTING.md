# Contributing to PAD

Thanks for helping out. This repository has a few structural rules that are enforced in CI and are
not obvious from the code — read this section first, because a PR that misses them fails CI before
anyone looks at the change itself.

Security problems go to `SECURITY.md`, not to the issue tracker.

## Repository layout

- `rust-tui/` — the crate. `pad` and the `pad-sider` panel live here.
- `install/` — installer modules; `install.sh` at the root is generated from them.
- `scripts/` — build helpers, hook bridge, CI checks.
- `docs/` — platform support, agent compatibility, release checklist.
- Every tracked directory has an `index.md` describing what is in it. Read the local `index.md`
  before reading the code, and update it when you add, move, or delete files.

## Hard rules enforced by CI

**1. Rust source files have role-based size limits.**
Tracked production files under `rust-tui/src/**/*.rs` may contain up to 500 lines, while external
test files (`*_tests.rs`, `tests.rs`, or files below a `tests/` directory) may contain up to 800.
No Rust file may exceed the absolute 1000-line limit. Only `rust-tui/src/i18n/` is exempt from the
role-based limit because translation tables are intentionally dense; the absolute limit still
applies. Prefer one cohesive module over several tiny forwarding files, then split by responsibility
when a file approaches its limit.

**2. Unit tests live in external files.**
Ordinary source files must not contain the literal `mod tests {`. Put the tests in a sibling file
whose name ends in `_tests.rs` (or under a `tests/` directory) and attach it from the source file:

```rust
#[cfg(test)]
#[path = "thing_tests.rs"]
mod tests;
```

Both rules are checked by:

```bash
python3 scripts/ci/check_rust_file_size.py
```

**3. Every directory needs a short `index.md`.**
Each tracked directory must contain an `index.md` of at most 50 lines. Any local path written in
backticks inside an `index.md` must actually exist, otherwise the check reports a stale ref. A new
directory means a new `index.md`, and a renamed file means updating the `index.md` that mentions it.

```bash
python3 scripts/ci/check_index.py
```

**4. `install.sh` is generated.**
Do not edit `install.sh` by hand. Edit the modules under `install/`, then regenerate and confirm the
result is committed:

```bash
bash scripts/build_installer.sh
git diff --exit-code -- install.sh
```

## Local verification

Run this before pushing; it is the same set CI runs first:

```bash
cd rust-tui && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

Dependency advisories are also checked in CI with `cargo audit`. To reproduce locally:

```bash
cargo install cargo-audit --locked
cd rust-tui && cargo audit
```

## Toolchain and MSRV

`rust-tui/rust-toolchain.toml` pins `stable`, so day-to-day development and CI always validate on the
current stable toolchain. `rust-version` in `rust-tui/Cargo.toml` records the minimum supported Rust
version for source builds; it is `1.88`, and it is driven by upstream crates (`ratatui`, `time`,
`darling`), not by code in this repository. Raising it is fine when a dependency needs it — say so in
the PR description, because it affects users who build from source on older distributions.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/), matching the existing history:

```
feat(relay): add real chat provider probes
fix(tmux): target writable client on handoff
perf(sidebar): reduce search allocations
```

Common types here: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `ci`, `chore`. The scope is the
subsystem (`relay`, `tmux`, `pad-sider`, `session-cache`, ...). Keep the subject in the imperative
and lowercase.

## Pull requests

- One logical change per PR. File limits are guardrails for cohesive modules, not a target size or a
  reason to split a focused change across many tiny files.
- Add or update tests for behavior changes, in an external `*_tests.rs` file.
- Update the affected `index.md` files and, for user-visible changes, add an entry under
  `[Unreleased]` in `CHANGELOG.md`.
- Fill in the PR checklist. It is the same list as above.

## Versioning and releases

Releases are maintainer-driven. The version policy (patch bumps, when a `hotfix` tag is acceptable)
is in `AGENTS.md`, and the pre-release, verification, and rollback steps are in
`docs/release-checklist.md`. Contributors do not need to bump the version in a PR.

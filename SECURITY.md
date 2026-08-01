# Security Policy

中文读者可先看 [README_ZH.md](README_ZH.md) 了解项目；安全问题请按下面的流程私下报告，不要开公开 issue。

PAD runs on your machine, launches other agent CLIs through tmux, and reads and writes agent
configuration under your home directory. A bug on that surface can expose API keys or let another
local process act as you, so we treat those reports privately rather than in the public tracker.

## Supported versions

| Version | Supported          |
| ------- | ------------------ |
| 0.7.x   | Yes                |
| < 0.7   | No                 |

There are no maintenance branches. A fix lands on `master` and ships in the next patch release, so
"upgrade to the latest release" is always part of the remediation. Version and release mechanics are
described in `docs/release-checklist.md`.

## Reporting a vulnerability

Report through a GitHub private security advisory:

<https://github.com/T1mn/pad/security/advisories/new>

(Or: repository page -> **Security** -> **Advisories** -> **Report a vulnerability**.)

Please do not open a public issue, pull request, or discussion for a suspected vulnerability, and
do not post a proof of concept publicly before a fix is out. If GitHub advisories are unavailable to
you, email the maintainer address in the `authors` field of `rust-tui/Cargo.toml` with `pad security`
in the subject.

Useful things to include:

- `pad --version`, OS, and `tmux -V`
- what an attacker gains, and what access they need to start with
- reproduction steps or a minimal proof of concept
- any config or state paths involved, with tokens redacted

## Response expectations

This is a small project with one maintainer, so these are best-effort targets, not an SLA:

| Stage                                | Target             |
| ------------------------------------ | ------------------ |
| First human reply                     | within 5 days      |
| Triage: confirmed / not a bug, severity | within 10 days   |
| Fix released for high severity        | within 30 days     |
| Fix released for lower severity       | within 90 days     |

We will tell you when the fix ships and credit you in the advisory and in `CHANGELOG.md`, unless you
ask to stay anonymous. Please hold public disclosure until the fix is released or the window above
has passed.

## Scope

In scope:

- the `pad` binary and its `pad-sider` panel
- `install.sh` and the installer modules under `install/`
- relay and provider configuration handling, including credentials written into agent config files
- the local control socket, hook bridge, and tmux integration
- release artifacts published from this repository

Out of scope, unless PAD is what makes them exploitable:

- bugs in tmux, in the agent CLIs (Codex, Claude Code, Grok Build, OpenCode, Gemini), or in upstream
  crates — report those to their maintainers, and tell us if PAD widens the impact
- anything that already requires code execution as the same user on the same machine
- missing hardening with no demonstrated impact, and automated scanner output without a scenario
- social engineering, physical access, and denial of service against your own terminal

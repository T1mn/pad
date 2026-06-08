# Telegram Pending Modules

- `pending.rs`: module entry and re-exports. Keep external call sites stable.
- `status.rs` / `status/`: pending state text, summaries, metadata lines and continuity formatting.
- `feedback.rs`: draft feedback gate, typing updates, and status refresh/finalize flow.
- `timeouts.rs`: hard timeout release for stuck Telegram requests.
- `journal.rs`: hook journal replay for recovery when direct hook delivery is missed.
- `failures.rs` / `failures/`: Codex rollout error polling, transcript scan and failure reply formatting.
- `results.rs`: result delivery, retry scheduling, and completion reply formatting.
- `approval.rs` / `approval/`: Codex approval scan loop, transcript resolution, state transitions and prompt notification.
- `timing.rs`: helper conversions for sent/accepted timestamps.
- `tests.rs`: module-focused tests for failure polling and continuity rendering.

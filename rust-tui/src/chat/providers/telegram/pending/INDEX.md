# Telegram Pending Modules

- `pending.rs`: module entry and re-exports. Keep external call sites stable.
- `status.rs`: pending state text, summaries, and continuity formatting.
- `feedback.rs`: draft feedback gate, typing updates, and status refresh/finalize flow.
- `timeouts.rs`: hard timeout release for stuck Telegram requests.
- `journal.rs`: hook journal replay for recovery when direct hook delivery is missed.
- `failures.rs`: Codex rollout error polling and failure message construction.
- `results.rs`: result delivery, retry scheduling, and completion reply formatting.
- `approval.rs`: Codex approval scan loop and approval-state transitions.
- `timing.rs`: helper conversions for sent/accepted timestamps.
- `tests.rs`: module-focused tests for failure polling and continuity rendering.

# hooks

- `../hooks.rs`：hook 子模块入口与对外 re-export。
- `direct.rs` / `direct/`：direct Unix socket listener、hook JSONL 读取与即时 pending 推进。
- `journal.rs`：hook journal 恢复时机、状态同步与事件去重。
- `pending_match.rs` / `pending_match/`：hook event 与 pending request 匹配、submit/stop 状态推进。
- `completion.rs` / `completion/`：pending stop 后的结果解析、缓存、投递结果记录与 Codex transcript catch-up。
- `tests.rs`：hook/pending turn 匹配与 Codex stop 结果解析回归测试。

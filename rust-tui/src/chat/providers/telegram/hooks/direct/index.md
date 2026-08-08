# telegram/hooks/direct

- `listener.rs`：direct hook Unix socket 状态检测、旧 socket 清理与 accept loop。
- `listener_tests.rs`：direct hook listener 的私有权限和冲突检测回归测试。
- `../direct.rs` 内联 `stream`：读取 socket JSONL stream，解析为 `HookEvent`。
- `process.rs`：根据 submit/stop hook 推进 pending request、刷新反馈与完成投递。

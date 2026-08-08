# hook

- `../hook.rs` 内联 `model`：Codex/agent hook socket 收到的事件与 tmux 上下文数据结构。
- `listener.rs`：Unix socket listener、连接处理和 JSONL 事件分发。
- `listener_tests.rs`：hook listener 的私有权限、冲突检测和同步 bind 回归测试。
- `../hook.rs` 内联 `journal`：将收到的 hook event 追加写入本地 journal，供回放/诊断使用。

# provider_test

- `probe.rs`：Provider 测试入口、credential 选择与 Codex/generic probe 分发。
- `../provider_test.rs`：单个与 Claude/Codex 批量 provider 测试调度，支持后台多结果回收。
- `client.rs`：HTTP client 构建与 bearer GET request helper。
- `generic.rs`：通用 provider reachability probe。
- `claude.rs` / `claude/`：Claude Messages 真实对话 probe，统计首字/完整耗时并分类错误。
- `codex.rs` / `codex/`：Codex Responses 真实对话 probe，统计首字/完整耗时并分类错误。
- `result.rs`：把 provider 测试结果写回配置状态，并清理 in-flight channel。
- `types.rs`：provider test channel message 与 probe outcome 类型别名。

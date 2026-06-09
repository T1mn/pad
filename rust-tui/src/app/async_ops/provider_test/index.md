# provider_test

- `probe.rs`：Provider 测试入口、credential 选择与 Codex/generic probe 分发。
- `client.rs`：HTTP client 构建与 bearer GET request helper。
- `generic.rs`：通用 provider reachability probe。
- `claude.rs`：Claude `/v1/models` relay probe 与 base URL normalization。
- `codex.rs`：Codex `/models` relay probe 与 base URL normalization 结果文案。
- `result.rs`：把 provider 测试结果写回配置状态，并清理 in-flight channel。
- `types.rs`：provider test channel message 与 probe outcome 类型别名。

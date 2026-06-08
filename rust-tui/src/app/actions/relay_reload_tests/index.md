# app/actions/relay_reload_tests

- `support.rs`：共享临时 HOME、provider 夹具、Codex provider seed/update 和 poll helper。
- `immediate.rs`：外部 relay 配置变更在非编辑态立即应用。
- `deferred.rs`：编辑态延迟 reload，结束编辑后应用。
- `invalid.rs`：非法外部配置忽略保护。
- `selection.rs`：reload 后 provider selection clamp 回归。

# relay_settings/tests

- `../tests.rs` 内联 `support`：共享临时 HOME、provider 夹具和 agent 查找 helper。
- `../tests.rs` 内联 `navigation`：settings host / standalone host 的 Esc 返回行为。
- `provider.rs`：provider 激活开关和 overlay 持久化测试。
- `opencode.rs`：OpenCode small model、models 弹层与 provider/model 引用更新测试。

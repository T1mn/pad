# app/actions/opencode_diagnostics

- `collect.rs` / `collect_tests.rs`：执行 OpenCode 诊断子命令并收集各 section 输出。
- `report.rs`：诊断报告格式化、敏感值脱敏，并以 `0600` 写入时间戳文件。
- `text.rs`：OpenCode diagnostics 动作 toast 文案。

# opencode_export

- `export.rs`：调用 OpenCode CLI export，检查 stdout 并写入导出文件。
- `../opencode_export.rs` 内联 `path`：根据 session id、导出目录和模式生成安全文件名。
- `../opencode_export.rs` 内联 `text`：导出成功/失败 toast 文案。
- `../opencode_export.rs` 内联 `mode`：raw / sanitized 导出模式。

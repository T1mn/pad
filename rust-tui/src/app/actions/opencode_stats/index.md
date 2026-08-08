# app/actions/opencode_stats

- `export.rs` / `export_tests.rs`：在选中项目目录执行 `opencode stats`、校验当前项目过滤参数与输出，并写入报告文件。
- `../opencode_stats.rs` 内联 `path`：OpenCode stats 导出文件名与时间戳路径。
- `../opencode_stats.rs` 内联 `text`：OpenCode stats 动作 toast 文案。

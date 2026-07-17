# claude_history

- `scan.rs`：Claude 历史扫描 facade，保持对外调用路径稳定。
- `scan/`：文件发现与索引同步实现。
- `parse.rs` / `parse/`：Claude JSONL 解析、首个用户 prompt 提取与时间解析。
- `api.rs`：对外查询入口，历史根目录跟随 `CLAUDE_CONFIG_DIR`。
- `db.rs`：数据存取 facade，保持对外导出路径稳定。
- `db/`：schema/open、查询、写入/归档实现。
- `model.rs`：历史数据模型。
- `util.rs`：公共辅助。
- `tests.rs` / `tests/`：按 parse、sync、archive 分组的测试。

# thread_meta

- `model.rs`：thread 元数据 key 与记录模型。
- `db.rs`：SQLite 路径、schema 初始化、迁移与连接辅助。
- `storage.rs` / `storage/`：thread 元数据、删除状态与标签的读写逻辑。
- `tests.rs`：schema 迁移、生成标题与删除状态测试。

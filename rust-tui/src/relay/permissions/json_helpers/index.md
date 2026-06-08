# relay/permissions/json_helpers

- `get.rs`：读取嵌套 JSON string/bool。
- `set.rs`：创建缺失对象并写入嵌套 JSON string/bool。
- `restore.rs`：按保存的旧值恢复，旧值不存在时移除路径。
- `remove.rs`：移除嵌套 JSON path，并清理空父对象。
- `cleanup.rs`：递归清理空 JSON object。

# relay/permissions/json_helpers

- `../json_helpers.rs` 内联 `get`：读取嵌套 JSON string/bool。
- `set.rs`：创建缺失对象并写入嵌套 JSON string/bool。
- `../json_helpers.rs` 内联 `restore`：按保存的旧值恢复，旧值不存在时移除路径。
- `../json_helpers.rs` 内联 `remove`：移除嵌套 JSON path，并清理空父对象。
- `../json_helpers.rs` 内联 `cleanup`：递归清理空 JSON object。

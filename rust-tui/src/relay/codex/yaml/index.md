# relay/codex/yaml

- `export.rs`：把当前 Codex provider 配置导出为 relay YAML。
- `import.rs`：读取 YAML 并映射回 `ProviderConfig`。
- `parse.rs`：解析当前导出格式的轻量 YAML。
- `string.rs`：YAML 字符串转义与反转义。
- `model.rs`：导入解析过程用的中间结构。

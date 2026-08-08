# relay/codex/yaml

- `../yaml.rs` 内联 `export`：把当前 Codex provider 配置导出为 relay YAML。
- `../yaml.rs` 内联 `import`：读取 YAML 并映射回 `ProviderConfig`。
- `../yaml.rs` 内联 `parse`：解析当前导出格式的轻量 YAML。
- `../yaml.rs` 内联 `string`：YAML 字符串转义与反转义。
- `../yaml.rs` 内联 `model`：导入解析过程用的中间结构。

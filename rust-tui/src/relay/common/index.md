# relay/common

- `paths.rs`：各 agent 原生配置、备份、状态文件路径；Claude settings 复用统一配置目录解析。
- `files.rs`：备份、恢复和容错写文件 helper。
- `formats.rs` / `formats/`：JSON/JSONC、TOML、env 解析与序列化。

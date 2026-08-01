# relay/common

- `paths.rs`：各 agent 原生配置、备份、状态文件路径；Claude settings 复用统一配置目录解析。
- `files.rs` / `files_tests.rs`：备份、恢复、私有原子写和容错写文件 helper 及测试。
- `formats.rs` / `formats/`：JSON/JSONC、TOML、env 解析与序列化。

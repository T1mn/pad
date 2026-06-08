# relay/codex

- `apply.rs`：Codex relay 配置应用流程，负责备份、恢复、写文件与 provider sync 触发。
- `provider.rs`：更新 Codex `config.toml` 的 `model_provider` / `model_providers`。
- `auth.rs`：更新 Codex `auth.json` 的 apikey auth 字段。
- `yaml.rs` / `yaml/`：Codex relay export/import YAML 入口、解析与 provider 映射。

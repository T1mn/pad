# relay

- `claude.rs`：Claude 原生配置写入、base URL 规范化、模型与兼容 env。
- `codex.rs` / `codex/`：Codex relay 应用入口，拆分配置写入、auth 更新、provider sync 与 export/import YAML。
- `deepseek.rs` / `deepseek/`：DeepSeek(cc) 独立 Claude 配置目录与启动脚本生成。
- `gemini.rs`：Gemini 原生配置写入。
- `opencode.rs` / `opencode/`：OpenCode 原生 provider、model 与托管状态写入。
- `permissions.rs` / `permissions/`：运行时权限、状态栏、prompt 覆盖。
- `common.rs` / `common/`：relay 配置通用路径、文件备份与格式解析工具。
- `tests.rs` / `tests/`：relay 与 runtime overlay 测试。

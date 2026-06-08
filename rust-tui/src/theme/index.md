# theme

- `../theme.rs`: 主题模块总入口，对外保持原来的 `crate::theme::*` 路径。
- `palette_core.rs`: `Theme` 结构和主题选择入口。
- `palette_dark.rs`: 默认和偏暗主题预设。
- `palette_light.rs`: 其余主题预设。
- `color.rs`: 主题颜色混合与可读性增强。
- `provider.rs`: relay provider 相关结构和 URL 规范化。
- `agent.rs`: agent / model 配置结构。
- `settings.rs`: 预览、声音、Telegram、Codex 等配置结构。
- `config.rs`: `Config` 主结构和默认值。
- `load.rs` / `load/`: 配置读取入口、TOML section 解析与 agent/provider 解析。
- `save.rs`: 配置保存。
- `tests.rs` / `tests/`: 按 config、sound、provider、palette 分组的主题回归测试。

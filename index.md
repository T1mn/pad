# PAD

- `apps/pad-desktop/`：macOS Desktop 主产品。
- `apps/pad-ios/`：iPhone 远程端。
- `apps/pad-terminal/`：固定 Ghostty 薄 fork + 原生工作台预览（cmux 风格 UI、可读字号/账号搜索、本地 Codex/Claude/Pi 会话接入、独立原生任务、单默认账号与 `~/.pad/preview` 显式迁移），不替换现有产品。
- `docs/`：当前产品架构、界面对齐、远程和发布说明。
- `scripts/ci/`：少量主路径 smoke/typecheck 工具。
- `.github/workflows/ci.yml`：TypeScript/React 主链 CI。

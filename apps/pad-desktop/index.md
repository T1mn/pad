# pad-desktop

- `electron/main/`：TypeScript 本地后端、SQLite、Pi RPC、跨 Session/子 Agent 协作、登录、模型、代理、终端和远程网关。
- `electron/preload/`：受限 IPC bridge。
- `renderer/src/`：React 的 macOS/Codex 风格界面。
- `shared/protocol/`：主进程与 renderer 的 protocol v2 类型。
- `Resources/`：图标和 Electron Utility Process 使用的 Pi host。
- `scripts/package-electron-app.sh`：打包 arm64 macOS App，固定 Pi/Electron 内置 Node.js 证据，并只保留中英文 locale。
- `scripts/install-electron-app.sh`：隔离健康检查、可恢复替换 `/Applications/PAD Desktop.app`，默认保留最近 3 份旧 App。
- `scripts/release-electron-app.sh`：生成 ZIP/DMG 发布物。

Desktop 不再包含 Rust、TUI、SwiftUI 回滚壳或 sidecar 兼容层。

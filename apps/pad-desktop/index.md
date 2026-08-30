# pad-desktop

- `electron/main/`：TypeScript 本地后端、SQLite、Pi RPC、登录、模型、代理、终端和远程网关。
- `electron/preload/`：受限 IPC bridge。
- `renderer/src/`：React 的 macOS/Codex 风格界面。
- `shared/protocol/`：主进程与 renderer 的 protocol v2 类型。
- `Resources/`：图标、Pi 启动器和 Bun/Node shim。
- `scripts/package-electron-app.sh`：打包 arm64 macOS App，并固定 Pi/Bun 运行时证据。
- `scripts/install-electron-app.sh`：隔离健康检查、可恢复替换 `/Applications/PAD Desktop.app`。
- `scripts/release-electron-app.sh`：生成 ZIP/DMG 发布物。

Desktop 不再包含 Rust、TUI、SwiftUI 回滚壳或 sidecar 兼容层。

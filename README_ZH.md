# PAD

PAD 是一个只面向 macOS 的 Pi 桌面客户端。它把 Codex 风格的任务侧边栏、Pi 对话、多账号、模型选择、快速模式、完全访问、本地终端和 iPhone 远程连接放进同一个 App。

## 当前架构

- Electron 主进程：TypeScript 本地后端，负责 SQLite、Pi 生命周期、登录、系统代理、终端和远程连接。
- React 界面：负责 macOS/Codex 风格交互，只接收安全的界面数据。
- Pi：唯一 Agent 内核，以 RPC 模式运行。
- 已取消 Rust sidecar、旧命令行 TUI 和 SwiftUI 回滚壳。

PAD 数据默认保存在 `~/Library/Application Support/PAD Desktop`。每个账号都有独立的 Pi 配置和会话目录，不会读取或覆盖 Codex、ChatGPT 的左侧会话数据。

## 开发

```bash
cd apps/pad-desktop
npm ci
npm run dev
```

## 验证、打包和安装

```bash
cd apps/pad-desktop
npm run typecheck
npm run test:ui -- --run
./scripts/package-electron-app.sh
./scripts/install-electron-app.sh --check-only
./scripts/install-electron-app.sh --launch
```

生成的 App 位于 `apps/pad-desktop/out/PAD Desktop-darwin-arm64/PAD Desktop.app`，安装位置固定为 `/Applications/PAD Desktop.app`。

## iPhone 远程端

`apps/pad-ios` 是原生 iOS 配套 App。在 Desktop 设置中开启远程连接，扫描一次性二维码后即可在同一局域网继续任务。

## 许可证

MIT

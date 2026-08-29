# PAD Desktop

- `Package.swift`：Swift Package Manager 构建入口，macOS 13+、零第三方依赖。
- `Sources/PADDesktopApp/PADDesktopApp.swift`：SwiftUI 应用、Codex 风格 Sidebar、对话区、Composer，以及 PAD `desktop-server` JSONL bridge。
- `Sources/PADDesktopApp/PiLogin.swift`：原生 macOS Pi 登录窗口与 Pi SDK JSONL 交互桥。
- `package.json` / `forge.config.ts` / `vite.*.config.ts`：Electron Forge + Vite + React/TypeScript 主应用；SwiftUI 入口仅作为回滚壳保留。
- `electron/main/`：单实例 macOS 窗口、CSP 和 PAD Rust sidecar 的唯一进程所有者；renderer 请求前强制 hello 协商 protocol v2，拒绝把旧 v1 私有记录透传给 Electron；远程设备在线时使用 `prevent-app-suspension` 保持系统活跃，同时允许显示器熄屏。
- `electron/preload/`：`contextIsolation` 下的 bootstrap/request/subscribe 白名单桥。
- `shared/protocol/`：renderer、preload 与 main 共用的 Desktop protocol v2 类型；包含统一 1 MiB 帧上限、安全记录、Rust-owned auth/terminal/UI-state/remote DTO、server event 与本地菜单事件契约。
- `renderer/src/components/RemoteSettingsSection.tsx` / `RemotePairingSheet.tsx`：中文远程连接设置、已配对设备管理，以及只在组件局部内存保存短期票据的 QR 配对 Sheet。
- `Resources/Info.plist`：应用包元数据模板。
- `scripts/package-app.sh`：构建 Swift release executable、Rust `pad` host，并生成内含控制面的可双击 `.app` bundle。
- `scripts/run-electron-forge.sh`：Forge 统一入口；只使用固定的 Node 24.20.0（本机或 npm 离线缓存），禁止构建阶段隐式下载工具链。
- `scripts/package-electron-app.sh`：校验本地 Electron ZIP、关键 Mach-O 的 macOS 13 deployment target，并把固定的 Pi 0.84.4、Bun 1.3.14 与 Rust `pad` 装入 arm64 Electron 应用；随包生成 runtime manifest、SPDX SBOM 和可复核 SHA-256，不覆盖 SwiftUI 回滚包。
- `scripts/release-electron-app.sh`：验证最终 `.app`、ZIP 与 DMG；默认只生成明确标记的 local-only ad-hoc 包，也支持显式 Developer ID + `notarytool` + App/DMG staple + Gatekeeper 评估路径，并输出版本化制品、runtime/公证证据与 SHA-256 清单。
- `scripts/install-electron-app.sh`：安装目标固定为 `/Applications/PAD Desktop.app`；先安全退出旧进程并生成可恢复备份，随后用隔离数据启动 renderer/backend protocol-v2 health probe，只有无 fatal alert 且全进程可回收才提交安装，失败自动回滚；支持 `--check-only` 无写入校验。
- `README.md`：开发、运行、打包和 Pi 配置说明。

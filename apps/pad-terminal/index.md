# PAD Terminal

Ghostty 薄 fork + SwiftUI 原生工作台实验线；不替换 `pad-desktop`，账号/任务数据隔离，本地 CLI 会话按显式操作只读接入。

- `upstream.lock.json`：固定 Ghostty tag、commit 与工具链版本。
- `scripts/upstream.py`：预检、源码拉取、补丁校验和本机构建入口。
- `scripts/setup-toolchains.py`、`dependencies.py`：固定工具引导和校验后恢复失败依赖下载。
- `scripts/zig-sdk/xcrun`：仅为 Zig 指定兼容的 macOS SDK，不改系统 Xcode 选择。
- `scripts/notices.py`：收集上游及依赖许可到预览 App。
- `scripts/pi-session.mjs`：独立 profile 的 Node/Pi CLI 入口，不依赖 Electron。
- `scripts/migrate-data-root.mjs`：关闭使用 PAD 根的客户端后，显式 `--confirm-closed` 迁移至 `~/.pad/preview`；不自动迁移/合并，旧源保留。
- `scripts/pi-session.test.mjs`、`pi-smoke.mjs`：入口隔离回归与零模型请求的 RPC 主路径。
- `native/`：cmux 风格原生界面、状态层、JSONL transport 和消息归一化；Swift 同步到受控 overlay，视觉默认配置另行打包。
- `native/DESIGN.md`：cmux 公开截图参照、仅复刻 UI 的边界及原生接入。
- `host/`：Node 后台、任务持久化、隔离 Pi RPC 和 SDK 认证；`openai-model-catalog.mjs` 提供显式官方 SIWC 账号模型同步与私有离线缓存，不回退静态 OAuth 表；`http-client.mjs` 在 SDK 前初始化公共已安装 Pi Undici，`auth-diagnostics.mjs` 提供有界安全 OAuth 分类，`PROTOCOL.md` 定义接口；原生 `PADProxyEnvironment.swift` 有限适配系统代理并警告不支持的路由。
- `scripts/workbench-smoke.mjs`、`native-smoke.swift`：工作台离线主路径及 Swift transport/流式夹具。
- `scripts/readability-smoke.swift`、`host/local-sessions.test.mjs`：搜索/DTO/argv 与本地会话格式、只读刷新、Pi 副本的离线回归。
- `patches/`：可审阅的最小上游改动，不改终端解析或渲染核心。
- `README.md`：开发命令、新根/显式迁移、单默认账号与历史 profile 保留、认证策略及剩余范围。
- `VALIDATION.md`：真实构建/smoke 证据、官方账号模型离线回归与 `out/upstream-models/` 原生增量产物、未验证真实端点和人工验收项。
- `.cache/`、`out/`：忽略的上游源码、构建产物；不进入正式 release。

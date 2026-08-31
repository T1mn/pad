# PAD Desktop 更新记录

## Unreleased

## 0.7.7

- Desktop 后端全部迁移到 Electron 主进程的 TypeScript，实现 SQLite、Pi RPC、登录、模型、Fast、完全访问、终端和远程连接。
- 删除 Rust sidecar、旧 TUI、SwiftUI 回滚壳、兼容 client 以及不再进入产品的旧测试和打包入口。
- 打包产物不再包含 `Contents/Resources/pad`，只包含 Electron、Pi、Bun 和应用资源。
- 远程网关改为与现有 iPhone App 一致的 WSS、证书指纹、一次性配对和 device token 协议。
- 新增任意 PAD Session 的查询、历史读取、消息投递和对话重命名。
- 新增 Codex 风格的持久化子 Agent：创建、继续、等待、中断和 Agent 树查询；子 Agent 在侧边栏显示为父任务的子项。

## 0.7.6

- 新增“设置 → 远程连接”，支持启停 Mac 端远程网关、显示实时在线数和管理已配对设备。
- 新增 PAD iOS 短期二维码配对 Sheet，支持倒计时、过期自动取消、Escape 退出与本地密钥清理。
- 新增 `remote_changed` 实时状态刷新；账号切换时清空旧状态并从控制面重新读取。
- 远程 DTO 严格丢弃 token、路径、监听端点与原始错误；renderer 不直接联网。
- 已连接设备在线时使用 `prevent-app-suspension`，连接归零或退出应用后立即释放。
- 保活信号覆盖所有 Profile 的真实在线连接；切换账号不会误释放，同时全局在线信号不会传入 renderer。
- App 包声明本地网络用途和 `_pad-remote._tcp` Bonjour 服务。

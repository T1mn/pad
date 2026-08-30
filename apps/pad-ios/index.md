# PAD Remote iOS

- `PADRemote.xcodeproj/`：共享 `PADRemote` scheme；应用与 XCTest target，iOS 17+，无第三方依赖。
- `PADRemote/App/`：SwiftUI 应用入口与 scene 生命周期。
- `PADRemote/Models/`：Desktop Remote v1 配对 URI、帧 codec、动作白名单与 1 MiB 上限。
- `PADRemote/Networking/`：`pad.remote.v1` WebSocket、叶证书 DER SHA-256 钉扎和 connection generation 隔离。
- `PADRemote/Persistence/`：仅设备且仅解锁可读的钥匙串、有界离线快照和原子命令发件箱；不存配对 secret。
- `PADRemote/State/`：断线重连、实时 Pi token 流、串行 outbox/ACK、epoch/revision、gap/resync 与 Desktop 快照兼容层。
- `PADRemote/Views/`：中文配对、二维码扫描、任务侧边栏、实时对话、Pi 交互卡片和 Composer。
- `PADRemoteTests/`：配对、codec、revision、jitter、钥匙串替身、持久化/跨配对隔离、流式 reducer 和交互 XCTest。
- `Info.plist`：相机、本地网络与 `_pad-remote._tcp` Bonjour 权限声明。
- `README.md`：连接、构建、测试、安全模型和 iOS 后台边界。

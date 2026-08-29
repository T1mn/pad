# apps

- `pad-desktop/`：macOS Desktop；Electron/React 为主应用，SwiftUI 仅作回滚壳；均通过随包的 `pad __internal desktop-server` 使用 PAD 私有 Store、Pi RPC 和会话历史。
- `pad-ios/`：原生 SwiftUI `PAD Remote`（iOS 17+）；通过证书指纹钉扎的 `pad.remote.v1` WSS 连接 Desktop，提供实时任务列表、对话和断线恢复，不暴露账号、终端、完全访问或目录选择。

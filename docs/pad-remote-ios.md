# PAD Remote for iOS

PAD Remote 是 PAD Desktop 的原生 iPhone 配套端。Desktop 的 TypeScript 后端仍然拥有 Profile、Task、Pi 进程、会话历史和权限；iPhone 只做任务列表、对话、发送、停止和必要交互。

## 连接流程

1. Mac 在设置中开启 Remote。
2. Desktop 创建一次性配对链接和二维码。
3. iPhone 扫码并提交设备名、配对 ID 与 secret。
4. Mac 返回 device ID 和 device token；iPhone 存入 Keychain。
5. 后续连接用 device ID/token 恢复，Mac 可随时撤销设备。

## 产品原则

- 同一局域网优先，低延迟 WebSocket 常连接。
- 前台实时连接；进入后台时保存缓存并正常断开，回前台快速恢复。
- 账号登录、模型配置、目录选择、终端和完全访问只在 Mac 上操作。
- 远程消息限制为 1 MiB，支持 ping/pong 和断线重连。
- PAD Remote 不读取 Codex/ChatGPT 的本地数据。

## 代码

- Desktop：`apps/pad-desktop/electron/main/remote-gateway.ts`
- iOS：`apps/pad-ios/PADRemote/`
- iOS 类型检查：`scripts/ci/pad_ios_typecheck.sh`

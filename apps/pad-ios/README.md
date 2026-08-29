# PAD Remote for iOS

PAD Remote 是 PAD Desktop 的原生 iPhone / iPad 实时遥控端。它只传输任务列表、对话历史、消息与任务控制；账号登录、终端、目录选择和“完全访问”始终留在 Mac 上。

## 快速连接

1. 确保 Mac 上的 PAD Desktop 0.7.6 或更新版本正在运行。
2. 在 Desktop 的“远程连接”中开启服务并生成一次性二维码。
3. iPhone / iPad 与 Mac 位于同一局域网；当前版本仅支持二维码中的 LAN 地址直连。
4. 打开 PAD Remote，扫描二维码；也可以粘贴完整的 `pad://remote/pair?...` 链接。
5. 首次连接成功后，设备令牌只保存在 iOS 钥匙串，并且仅在设备解锁时可读；任务缓存和发件箱保存在应用私有目录。

二维码中的一次性密钥不会写入 UserDefaults、文件或日志。WSS 连接只在二维码中的 SHA-256 指纹与 Mac 的叶证书 DER 完全一致时建立，自签名证书不享有任何例外。

## 实时与恢复策略

- 前台立即连接；前六次断线恢复使用 0.25 / 0.5 / 1 / 2 / 4 / 8 秒上限的 full-jitter，持续不可达后降为 30–60 秒冷却重试。
- 网络从不可用变为可用、应用重新进入前台时，立即跳过等待并恢复。
- 命令先以稳定 UUID 原子写入有界发件箱，再由单一串行发送器按序发送；明确结果后删除，断线重发时服务端可按 UUID 去重。
- 事件按 `server_epoch + revision` 去重、检测缺口并 ACK；缺口触发侧边栏和当前历史快照同步。
- 旧 WebSocket generation 的迟到回调会被丢弃，不能覆盖新连接状态。

## iOS 后台边界

iOS 进入后台后会申请短时系统后台任务完成缓存落盘，并主动关闭 WebSocket；发件箱命令在 UI 显示已发送前已经持久化。返回前台时先显示缓存，再发送 `resume` 恢复增量事件。这是遵循 iOS 生命周期的可靠策略，不宣称后台永久保活。

当前 P0 已实现同一局域网直连。Tailscale / VPN 需要 endpoint 地址重写与实网验证，暂不作为已交付能力承诺。尚未实现 APNs 唤醒或公网中继；离开可达网络时，新命令保留在本机发件箱，回到前台且网络可达后发送。

## 协议边界

- WebSocket 子协议：`pad.remote.v1`
- 文本 JSON 帧最大 1 MiB
- 移动端动作白名单：`bootstrap`、`list_sidebar`、`history`、`create_task`、`start_task`、`prompt`、`abort`、`stop`、`stop_task`、`retry_task`、`respond_ui`、`set_task`、`runtime_snapshot`
- 移动端不存在 auth、account、terminal、full-access 或 cwd/path 选择 API

稳定 UUID 与 Mac 端 receipt 可以覆盖正常断线重放；这不宣称跨进程绝对 exactly-once。`create_task` 额外携带 iOS 预生成的稳定 `task_id`，避免 receipt 提交窗口内的 Mac 崩溃产生第二个任务；其他 mutation 的最终幂等边界仍由 Mac 服务端负责。

## 构建与测试

要求 Xcode 26 或兼容版本、iOS 17 SDK。项目没有第三方依赖。

在仓库根目录可先运行不依赖可启动 Simulator 的三目标类型检查：

```bash
./scripts/ci/pad_ios_typecheck.sh
```

```bash
cd apps/pad-ios
xcodebuild -project PADRemote.xcodeproj \
  -scheme PADRemote \
  -destination 'generic/platform=iOS Simulator' \
  CODE_SIGNING_ALLOWED=NO build

xcodebuild -project PADRemote.xcodeproj \
  -scheme PADRemote \
  -destination 'generic/platform=iOS' \
  CODE_SIGNING_ALLOWED=NO build
```

安装了 iOS Simulator runtime 时可运行：

```bash
xcodebuild -project PADRemote.xcodeproj \
  -scheme PADRemote \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  CODE_SIGNING_ALLOWED=NO test
```

单元测试覆盖配对 URI、1 MiB codec、revision duplicate/gap/epoch、重连与长期冷却、可替换钥匙串接口、发件箱原子写入/有界/UUID 幂等、跨配对隔离、实时 token 流、乐观消息对账、交互卡片和嵌套历史文本解析。

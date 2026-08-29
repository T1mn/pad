# PAD Desktop 更新记录

## 0.7.6

- 新增“设置 → 远程连接”，支持启停 Mac 端远程网关、显示实时在线数和管理已配对设备。
- 新增 PAD iOS 短期二维码配对 Sheet，支持倒计时、过期自动取消、Escape 退出与本地密钥清理。
- 新增 `remote_changed` 实时状态刷新；账号切换时清空旧状态并从控制面重新读取。
- 远程 DTO 严格丢弃 token、路径、监听端点与原始错误；renderer 不直接联网。
- 已连接设备在线时使用 `prevent-app-suspension`，连接归零或退出应用后立即释放。
- 保活信号覆盖所有 Profile 的真实在线连接；切换账号不会误释放，同时全局在线信号不会传入 renderer。
- App 包声明本地网络用途和 `_pad-remote._tcp` Bonjour 服务。

# PAD Desktop 成型计划

## 目标

做一个可直接使用的 macOS App：界面层级对齐 Codex，底层直接使用 Pi，不再维护 Rust sidecar、旧 TUI、SwiftUI 回滚壳和兼容入口。

## 最终数据流

```text
React Renderer
  -> typed preload IPC
  -> Electron main / TypeScript local backend
     -> PAD SQLite
     -> Pi ModelRuntime 登录与模型目录
     -> Pi RPC task process
     -> macOS system proxy
     -> local terminal
     -> iPhone remote gateway
```

Renderer 不接触凭据、SQLite 文件或 Pi 子进程。PAD 数据根与 Codex/ChatGPT 数据根独立。

## 工作阶段

| 阶段 | 交付 | 验收 |
|---|---|---|
| 1. 后端迁移 | SQLite、Profile/Project/Task、UI state 改为 TypeScript | 旧数据库可打开，侧边栏和选中状态可恢复 |
| 2. Pi 直连 | 登录、模型目录、聊天、历史、停止、重试改为 Electron 直连 Pi | 不启动 Rust 进程即可完成一轮假 Pi 对话 |
| 3. 产品能力 | Fast 默认开启、Full Access 自动确认、系统代理继承 | UI 设置与实际 Pi 请求一致 |
| 4. 远程连接 | Desktop 网关、一次性配对、设备恢复和撤销 | iPhone 可连接、收任务并发送消息 |
| 5. 瘦身 | 删除 Rust/TUI、SwiftUI 回滚壳、sidecar client、旧安装器和旧 CI | 仓库与 App 均无 Rust 构建入口和 `Resources/pad` |
| 6. macOS 成型 | 打包、隔离启动、安装并打开 | Renderer/backend ready、无 fatal alert、干净退出 |
| 7. UI 打磨 | 侧边栏、Composer、活动区、设置页对齐 Codex 层级 | 宽窄窗口无错位，中文文案完整 |

## 当前完成情况

- TypeScript SQLite 数据层已接管现有 schema。
- Electron 主进程已直接管理 Pi RPC、登录和模型目录。
- Fast、Full Access、系统代理、会话恢复和基础终端已接入。
- Remote gateway 已进入 TypeScript 主进程。
- Rust/TUI/SwiftUI 与旧兼容入口已从产品树删除。
- `.app` 已能在无 Rust sidecar 的情况下打包并通过隔离健康检查。

## 当前阶段只保留的测试

1. TypeScript typecheck。
2. Renderer 交互回归。
3. 一条本地后端主链：启动、新建任务、发送、历史、远程配对、退出。
4. 打包后真实 App 健康检查。

账号切换、真实模型选择、真实对话和 UI 对齐属于下一轮产品验收，不扩展成大量防御性边界矩阵。

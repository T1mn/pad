# PAD Desktop

PAD Desktop 是 Electron + React + TypeScript 的原生感 macOS App，Pi 是唯一 Agent runtime。

## 用户主流程

1. 从左下角账号菜单新增或切换 Profile。
2. 在可视化登录页完成 Pi Provider 登录。
3. 新建任务并选择工作目录。
4. 从 Composer 选择真实模型和思考强度；快速模式默认开启。
5. 发送消息后，Pi RPC 事件直接进入活动区和会话历史。
6. 完全访问开启时，PAD 自动处理 Pi 的确认请求。
7. 在设置中开启远程连接，让 iPhone 继续当前任务。

## 本地数据

默认数据根：`~/Library/Application Support/PAD Desktop`

- `v1/store/pad.sqlite`：Profile、Project、Task、UI state。
- `v1/profiles/<profile>/agent/`：该账号独立的 Pi 登录与模型配置。
- `v1/profiles/<profile>/sessions/`：该账号独立的 Pi 会话。
- `v1/remote/`：远程开关和已配对设备。

这些路径与 Codex/ChatGPT 数据目录完全独立。

## 核心代码

- `electron/main/local-backend.ts`：Renderer 请求入口。
- `electron/main/local-store.ts`：SQLite 数据层。
- `electron/main/pi-runtime.ts`：Pi RPC 生命周期、Fast、Full Access、历史。
- `electron/main/pi-sdk.ts`：登录与模型目录。
- `electron/main/remote-gateway.ts`：iPhone 实时连接。
- `renderer/src/`：Codex 风格界面。
- `shared/protocol/`：主进程、preload、renderer 共用协议。

## 开发与验收

```bash
npm ci
npm run dev
npm run typecheck
npm run test:ui -- --run
./scripts/package-electron-app.sh
./scripts/install-electron-app.sh --check-only
./scripts/install-electron-app.sh --launch
```

当前阶段只保留产品主链验收：启动、账号、模型、聊天、重启恢复、侧边栏、代理、远程入口和干净退出。

# Native host

隔离的 Node JSONL 子进程；不依赖 Electron，不共享正式账号或任务数据库。

- `PROTOCOL.md`：原生工作台协议与错误/认证边界（冻结，外部约定）。
- `workbench-host.mjs`：入口；先清除 ambient 模型凭据/NODE_OPTIONS，再加载模块；stdin/stdout 帧、请求分派、退出与信号处理。
- `workbench.mjs`：命令实现；workspace/profile/task 持久化、每任务 Pi 进程编组、history/选模型/prompt/abort。
- `workbench-auth.mjs`：同步占位的认证状态机、attemptId、原生提示/通知、取消与登录退出互斥。
- `workbench-store.mjs`：`v1/workbench.json` 私有元数据、单实例锁、原子保存与损坏拒绝。
- `pi-sdk.mjs`：按 profile 懒加载 Pi SDK `ModelRuntime`，账号/catalog 仅用显式 auth/models 路径，`refreshOnCreate:false`、`allowModelNetwork:false`；OpenAI OAuth 目录不回退静态表，API Key/其他 provider 保留 SDK 目录。
- `openai-model-catalog.mjs`、`openai-model-catalog.test.mjs`：显式同步官方 SIWC `models[]` 账号元数据、私有离线/stale 缓存、凭据变更失效与 runtime 不支持 slug 禁用；fake SDK/fetch 回归。
- `pi-task.mjs`：单个 Pi RPC 子进程封装与去重启动；LF 分帧、请求关联、事件透传、未知 extension UI 一律 cancel。
- `local-sessions.mjs`、`local-session-formats.mjs`：标准目录的只读会话发现/有界历史，缓存、路径校验与原 CLI argv 准备。
- `local-session-import.mjs`：显式复制 Pi 完整会话到私有任务目录；不改源文件、不复制凭据。
- `host-env.mjs`：环境清洗、`~/.pad/preview` / 显式覆盖的数据根与已安装 Pi 包发现（只读，不安装）。
- `data-root-migration.mjs`：默认启动根校验、独立桥租约与显式离线迁移（源保留，目标排他发布）。
- `installation-id.mjs`：每 PAD 根稳定 UUID 的懒加载同步回调、私有存储与无覆盖发布。
- `auth-policy.mjs`：SDK 可调用方法、订阅 OAuth 排除策略与安全错误分类。
- `auth-diagnostics.mjs`：显式 OpenAI OAuth 的白名单诊断，尽力写入 `<root>/v1/auth-diagnostics.jsonl`（100 条 / 64 KiB，私有权限、拒绝符号链接）；不恢复历史失败，阶段可能 unknown。
- `http-client.mjs`、`http-client.test.mjs`：公共 Undici HTTP 初始化、显式 HTTPS 空值直连分流、NO_PROXY 与有界自有 agent 清理；离线 mock dispatcher 回归。系统代理拒绝以固定 `host_warning` code 通知原生 UI。
- `jsonl.mjs`：严格 LF 分帧与 8 MiB 帧预算。
- `*.test.mjs`：离线针对性测试（fake Pi / 临时目录，无模型请求）。

只在用户点击发送后发起模型请求；认证仅由用户明确发起。后台 EOF/退出应取消认证并关闭自己创建的 Pi 子进程。

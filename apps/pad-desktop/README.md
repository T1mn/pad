# PAD Desktop（macOS）

PAD Desktop 是 PAD 的 macOS 原生壳：底层保留 Pi 的极简 RPC 工作流，界面采用 Codex 风格的 macOS 工作台（产品入口、新建任务、项目/最近任务、账号区、任务标题栏、对话流、工作区面板和多行输入卡片），不读写 ChatGPT 或 Codex 的 Session 数据。SwiftUI 只做渲染，所有持久化、权限和 Pi 进程均由随 App 打包的 `pad __internal desktop-server` 控制面负责。

当前实现使用 SwiftUI 和 Swift Package Manager，无第三方依赖，最低支持 macOS 13。App 启动后通过 stdin/stdout JSONL 拉起随包的 `pad __internal desktop-server`；SwiftUI 不直接打开 SQLite 或启动 Pi。开发运行时也可以通过 `PAD_BIN=/absolute/path/to/pad` 指定控制面。

界面默认完全使用简体中文（包括菜单、侧边栏、状态提示、权限提示和输入引导），并将窗口的系统 locale 固定为 `zh_CN`。Pi 返回的模型内容保持原样，因此可以在同一任务中自由使用中文或其他语言。

## 快速启动

```bash
cd apps/pad-desktop
swift run
```

生成可双击的 macOS App：

```bash
cd apps/pad-desktop
./scripts/package-app.sh
open "dist/PAD Desktop.app"
```

如果希望使用非默认 PAD host，可在开发运行时指定 `PAD_BIN=/absolute/path/to/pad`；打包脚本默认会编译并内置 `rust-tui/target/release/pad`。如果构建机已安装 Pi，脚本会把 Pi package 放入 App 的 Resources，并优先使用 Bun arm64 runtime；这样在干净 Mac 上不依赖构建机的 Homebrew Node 动态库。`Resources/bin/node` 是给原生登录桥使用的兼容 shim，实际调用同一个 Bun runtime。若构建机没有 Bun，则回退到 Node/PATH，并在发布前应补齐目标机 runtime 依赖。首次使用仍需在 Profile 的原生登录窗口配置 provider 凭据。

运行时会把 Profile 的 Pi 配置和 Session 放到：

```text
~/Library/Application Support/PAD Desktop/v1/profiles/<profile-id>/pi-agent
~/Library/Application Support/PAD Desktop/v1/profiles/<profile-id>/pi-sessions
```

不会把路径指向 `~/.codex`、ChatGPT 容器或用户独立的 `~/.pi` Session。

## 现有能力

- Codex 风格侧边栏：Pi 账号/Profile 切换、任务筛选、项目树、最近任务和搜索；归档与置顶通过任务标题栏操作。
- Task 生命周期：新建草稿、首次发送时创建、选择、Pin、Archive、Unread 清除；点击“新建任务”不会留下空 Task。
- 原生对话界面：用户消息、Pi 消息、滚动和 Enter 发送。
- 运行态操作：运行/生成/工具执行/重试/审批/输入/失败状态；运行中发送按钮切换为停止，失败状态切换为重试。
- 项目和工作目录：侧边栏可直接添加本地目录为 Project，新任务可以绑定已有 Project；附件按钮使用原生文件选择器，所选路径会随提示词真实发送给 Pi。
- 模型设置：Composer 使用当前账号的 Pi 服务商，可选择 Pi 自动模型与思考强度；运行中的设置通过 Desktop bridge 下发给 Pi。
- Codex 风格工作区面板：查看输出占位、当前任务运行状态、消息数、Profile 和权限策略。
- Pi 账号登录：从底部账号菜单打开原生 macOS 登录窗口；添加账号时先完成命名和服务商选择，再进入 Pi SDK 驱动的 OAuth、设备码或 API Key 登录，不打开 Terminal。每个 Profile 使用独立的 `pi-agent` 目录。
- Full Access 状态展示和快捷键 `⇧⌘F`；切换通过 `set_profile` 写入 PAD 私有 Store，权限执行策略由 PAD Rust control plane 负责。
- PAD desktop-server：JSONL 请求/响应、SQLite 私有 Store、Profile/Task 生命周期、Pi RPC 消息轮询。
- Pi RPC：由 Rust host 按 Profile 隔离启动；Pi 未安装或未配置凭据时，UI 仍可打开并展示明确错误。历史、流式消息、approval/select/input/editor UI、停止和思考强度均通过 Desktop bridge 的真实 RPC 链路处理。
- 独立 App 数据根：不触碰 Codex / ChatGPT 的 Sidebar 和 Session。

## 设计边界

App 使用 `Contents/Resources/pad` 作为唯一控制面入口：Profile、Project、Task 和 Pin/Archive 标记写入 `~/Library/Application Support/PAD Desktop/v1/store/pad.sqlite`；Pi 的 agent root 和 session journal 位于同一数据根的 Profile 子目录。不会访问或覆盖 `~/.codex`、ChatGPT 容器或用户原有 Pi 根目录。

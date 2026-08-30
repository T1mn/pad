# PAD Desktop 的 Codex 对齐基线

PAD 借鉴 Codex macOS App 的信息层级与交互密度，但 Agent 内核是 Pi。

## 窗口层级

1. 左侧平铺 Sidebar，不做悬浮卡片。
2. 中间是任务标题、对话时间线和底部 Composer。
3. 右侧检查器只在用户打开活动、文件或更改时占位。
4. 设置和登录使用原生感 Sheet，不跳到终端完成。

## Sidebar

- 顶部：新任务、搜索、需要关注。
- 中部：全部/置顶/归档，随后是 Project -> Task 的稳定树。
- 底部：当前账号与设置。
- 多账号切换只替换 PAD 当前 Profile 的任务，不合并 Codex/ChatGPT 会话。

## Composer

- 输入是首要焦点。
- 模型显示真实 provider/model 名称；未加载时只显示 Auto。
- 思考强度与 Fast 是独立选项。
- 完全访问是清晰的 Profile/Task 策略，不反复弹确认框。

## 活动区

Pi 的文本、工具、状态、确认和错误按时间顺序进入同一时间线。连接错误需要给出可行动原因，不能让任务永久停在“运行中”。

## 实现边界

- Sidebar 顺序由 TypeScript store 的 canonical snapshot 决定。
- React 不自行重建第二套 Profile/Project/Task 树。
- Electron main 是唯一 SQLite writer 和 Pi session owner。
- UI state、Profile 数据、Pi 凭据及会话全部位于 PAD 数据根。

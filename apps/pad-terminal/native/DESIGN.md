# Workbench visual baseline — cmux

用户要求：只复刻 cmux 的 UI，PAD 的任务/profile/原生 Pi 会话仍是自身产品逻辑。

## 参照

- https://cmux.com/ （2026-10-04 首页桌面截图）
- https://github.com/manaflow-ai/cmux/blob/main/docs/customizing-appearance.md （字号默认值与可调节性；只参考文档）
- https://github.com/manaflow-ai/cmux/blob/main/docs/assets/vertical-horizontal-tabs-and-splits.png
- 本地参考图：`.cache/cmux-reference/{landing,splits}.png`，仅用于对照，不随 App 分发。
- 根据公开截图独立实现；不复制 cmux 源代码、品牌图标、截图素材或功能架构。cmux app 源码当前为 GPL-3.0-or-later，本轮不引入该依赖。

## UI 规格

- 顶部 34pt 一体化窄工具栏：左侧真实 macOS 红黄绿灯预留 80px；其后侧栏/任务动态/新建等可操作图标；右侧文件夹 + 当前项目/任务标题。
- 左侧约 280pt，深炭灰，全高垂直列表。项目是小型分组标题；任务是紧凑 3 行纵向标签（名称、真实状态/账号、目录），选中项完整蓝色背景、4px 圆角。不要系统浅灰大 sidebar / 大标题 / 卡片堆叠。
- 右侧默认左右两个平铺 pane：PAD 原生 Agent 与 Ghostty 终端。以 1px 分隔线、34pt tab strip 划分，活动标签细蓝顶线，不堆叠标题区。
- 不画不可用的浏览器、PR、git branch、端口、未读数字或假 tab；状态全部来自现有数据。工具栏按钮必须真实可操作或不显示。
- 会话用密集、左对齐的日志式排版；用户输入/工具行轻底色矩形、无左右聊天气泡。字体 SF Mono 14pt 起，内容留白小而整齐。
- 可用性修订：cmux 官方外观文档的侧栏默认 12.5pt、tab 11pt，并支持整体缩放；不再把 PAD 主文字压到 9–11pt。正文默认 14pt、次要文字通常 ≥12pt，原生「aA」菜单支持正文 14/16/18pt；终端默认 14pt，沿用独立字号快捷键。
- 登录页搜索固定在列表上方，支持别名/已登录筛选和常用快捷入口；搜索不会登录，Return 不关闭账号页。
- 侧栏新增本地会话入口：只读同步、搜索、原工具续接；Pi 副本导入不等于双向同步，不改本地凭据。
- 输入区紧凑 3–5 行，底部状态行含模型/账号入口及发送/停止；普通 Return 仍只换行，发送仅显式点击。
- 共用色板 `PADWorkbenchStyle`：暗色 sidebar #20211f / canvas #262722 / chrome #232420 / border #3a3b36 / text #d8d8d4 / muted #a4a79f / accent #3478f6；提供对应浅色模式。
- 账号 sheet 只调整视觉密度/色板，保留所有 API Key/OAuth、取消、SecureField、URL 安全和认证状态逻辑。

## 实施边界

- `PADWorkbenchStyle.swift`：共用色板/图标按钮；`TerminalController` + `TerminalViewContainer` 在创建 hosting view 前配置透明全尺寸标题栏，macOS 14+ 关闭 hosting safe-area 重复留白；透明窗口装饰对该受控容器清除标题栏底色，避免遮住顶栏控件。
- `PADWorkbenchView.swift`：侧栏、工具栏、横向 pane 布局与终端 tab strip。
- `PADConversationView.swift` / `PADAccountView.swift`：会话与账号的平面紧凑风格。
- cmux 视觉阶段不改业务层；后续本地会话增量只扩展 Model/host 的显式命令，不改 transport / reducer / Desktop / iOS，仅运行相关离线回归。
- `PADDefaults.ghostty` 在 Swift 配置入口先加载，仅控制终端配色/字号/边距，用户设置和 CLI 可覆盖。
- 保持 TerminalContent 结构稳定；侧栏隐藏/任务切换都不得重建 PTY。入口增加可选 `splitTerminal: (Bool)->Void`，true=下方，false=右方，由上游现有 binding action 执行。
- 主 agent 一次整合 Swift typecheck、必要的原生增量构建、一条窗口截图/交互主路径；子 agent 不构建或重复测试。

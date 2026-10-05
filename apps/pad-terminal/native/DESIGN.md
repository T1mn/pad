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
- 侧栏本地会话入口「本地会话 · Codex / Pi」：仅 Codex/Pi 的只读同步、搜索、原工具续接，不发现/预览/续接 Claude；Pi 副本导入不等于双向同步，不改本地凭据。
- 输入区紧凑 3–5 行，底部状态行含模型/账号入口及发送/停止；仅输入区 Enter / ⌘Enter 发送、Shift+Enter 换行，也可点击发送；输入法 marked text 的 Enter 确认交回 AppKit，不发送。
- 模型入口锚定搜索 popover：名称/ID/Provider 本地部分匹配，保留上游顺序/分组；搜索 Return 不选模型或发送。切换任务/账号/目录关闭，并在点击时检查捕获上下文。UI 只称「官方账号目录」或「内置模型目录 · 账号权限未验证」；技术来源仍按 `openai_account|sdk` 区分，不伪称账号授权。
- thinking 菜单来自当前配置模型的实际 SDK 能力，含受支持的 max；按任务保存，未选不猜默认。运行中/认证/配置/abort 互斥；已有 Pi 以 setter 后 `get_state` 实际级别为准。
- 「会话信息」popover 区分真实 Pi ID / PAD 任务 ID，展示目录、账号名和文件状态，仅显式复制；只读缓存状态或安全首行，不启动 Pi、不写数据。新任务在 Pi 创建前不承诺已有会话 ID/文件，切换/关闭丢弃迟到响应。
- 共用色板 `PADWorkbenchStyle`：暗色 sidebar #20211f / canvas #262722 / chrome #232420 / border #3a3b36 / text #d8d8d4 / muted #a4a79f / accent #3478f6；提供对应浅色模式。
- 账号 sheet 只调整视觉密度/色板，保留所有 API Key/OAuth、取消、SecureField、URL 安全和认证状态逻辑。

## 实施边界

- `PADWorkbenchStyle.swift`：共用色板/图标按钮；`TerminalController` + `TerminalViewContainer` 在创建 hosting view 前配置透明全尺寸标题栏，macOS 14+ 关闭 hosting safe-area 重复留白；透明窗口装饰对该受控容器清除标题栏底色，避免遮住顶栏控件。
- `PADWorkbenchView.swift`：侧栏、工具栏、横向 pane 布局与终端 tab strip。
- `PADConversationView.swift` / `PADAccountView.swift`：会话与账号的平面紧凑风格。
- cmux 视觉阶段不改业务层；后续本地会话增量只扩展 Model/host 的显式命令，不改 transport / reducer / Desktop / iOS，仅运行相关离线回归。
- `PADDefaults.ghostty` 在 Swift 配置入口先加载，仅控制终端配色/字号/边距，用户设置和 CLI 可覆盖。
- 保持 TerminalContent 结构稳定；侧栏隐藏/任务切换都不得重建 PTY。入口增加可选 `splitTerminal: (Bool)->Void`，true=下方，false=右方，由上游现有 binding action 执行。
- 本 controls/session 集成只运行指定的 fake Node 测试、两个 Swift 纯 helper smoke 和一次原生增量构建/产物检查，不追加全量 typecheck 或 GUI。真实中文 IME、popover 焦点/复制、thinking 真实 getter 与模型请求均待人工验收；键盘策略夹具不等于输入法验收。

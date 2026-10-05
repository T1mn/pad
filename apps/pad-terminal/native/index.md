# Native workbench

macOS-only SwiftUI 产品层，通过受控 Node 子进程连接 Pi；Ghostty 只负责终端。

- `PADWorkbenchTypes.swift`：原生 DTO 与 UI 消息类型，模型可选 source/selectable/thinkingLevels、task thinkingLevel、只读 SessionInfo 与 OpenAI discovery 状态。
- `PADWorkbenchModel.swift`：应用级状态、默认新任务账号/历史任务有效 profile、目录代次守卫、显式 OpenAI 同步与离线读取、thinking 保存/操作守卫、只读会话信息、固定窗口合流和认证交互。
- `PADHostTransport.swift`：受限环境的子进程、请求关联与 JSONL framing；固定代理警告 code 进入独立 model/UI 警告状态。
- `PADProxyEnvironment.swift`：显式代理环境优先；保守拒绝不支持的 macOS bypass/PAC 系统路由并通知 UI，不读取凭据。
- `PADMessageReducer.swift`：分内容块重建流式文本、工具状态和历史归一化。
- `PADWorkbenchView.swift`：cmux 风格纵向任务标签、窄顶栏与左右平铺的原生 Agent / Ghostty。
- `PADWorkbenchStyle.swift`：色板、可读字号/持久化字号菜单、图标按钮；AppKit 接入保留系统交通灯。
- `PADProviderSearch.swift`：纯本地 Provider 搜索、常用别名与排序。
- `PADLocalSessionsModel.swift`、`PADLocalSessionsView.swift`：仅 Codex/Pi 本地历史搜索/同步、原 CLI 续接确认和 Pi 副本导入。
- `PADDefaults.ghostty`：打包的终端视觉默认值，用户配置/CLI 可覆盖。
- `DESIGN.md`：公开 cmux 截图参照、视觉规格与只改 UI 的边界。
- `PADConversationView.swift`、`PADAccountView.swift`：对话/模型选择、官方目录来源/stale 状态与「同步 OpenAI 模型」按钮、runtime 不支持项禁用；单默认账号登录面板（无 profile 创建/选择器，历史 profile 保留）。
- `PADModelPicker.swift`、`PADModelSearch.swift`：原控件锚定的原生搜索 popover；名称/ID/Provider 本地多词部分匹配，不改变目录分组顺序；点击才选择，任务/账号/目录变化关闭并校验捕获上下文。内置目录不代表账号权限验证。
- `../scripts/model-search-smoke.swift`：纯过滤离线夹具（编译命令在文件头；不覆盖原生焦点/IME）。
- `PADWorkbenchUIHelpers.swift`：共用展示组件与文案。
- `PADSessionInfoView.swift`：显式会话信息 popover，区分 Pi 会话 ID / PAD 任务 ID、文件存在状态与用户点击复制；关闭/切换任务丢弃迟到响应。
- `PADComposerInput.swift`、`PADComposerKeyDecision.swift`：仅输入区的 AppKit 编辑器与纯键盘决策；Enter / ⌘Enter 发送、Shift+Enter 换行，marked text 交回输入法，沿用 Model 发送守卫。
- `../host/PROTOCOL.md`：Swift ↔ Node 的最小协议与协作边界。

开发源码由 `scripts/upstream.py` 同步到固定上游的 `Sources/Features/PAD/`，不修改终端解析/渲染内核。已实现开发预览，真实账号/模型链路尚待人工验收；见 `../VALIDATION.md`。

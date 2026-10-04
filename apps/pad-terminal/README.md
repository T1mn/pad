# PAD Terminal Preview

Ghostty 薄 fork 上的 macOS 原生工作台，不是 Electron Desktop 的替换版。界面按 cmux 的窄顶栏、蓝色纵向标签和密集分屏布局独立实现：左侧项目/任务，右侧并排放置原生 Agent 与可分屏 Ghostty 终端。不把 Pi TUI 当作产品界面，也不重写终端内核。

## 本地开发

以下命令在仓库根目录执行。当前工具引导支持 Apple Silicon；需要完整 Xcode 26、Metal Toolchain，构建前至少留 10 GiB 空间。

```sh
/usr/bin/python3 apps/pad-terminal/scripts/setup-toolchains.py
# 仅在缺少 Metal 时安装 Apple 的开发组件：
# xcodebuild -downloadComponent MetalToolchain
/usr/bin/python3 apps/pad-terminal/scripts/upstream.py doctor
/usr/bin/python3 apps/pad-terminal/scripts/upstream.py prepare
/usr/bin/python3 apps/pad-terminal/scripts/upstream.py check
/usr/bin/python3 -u apps/pad-terminal/scripts/upstream.py build
open "apps/pad-terminal/out/PAD Terminal Preview.app"
```

- 固定 Ghostty `v1.3.1` / `332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28`、Zig `0.15.2`、Nushell `0.116.0`；下载工具校验 SHA-256。
- Zig 先生成本机架构 GhosttyKit，随后调用上游 `macos/build.nu --configuration ReleaseLocal`；最多两个编译作业，不跑上游测试或 benchmark。
- `.cache/ghostty/` 是固定源码加可审阅补丁；`prepare` 另外同步 `native/*.swift` 到受校验的 PAD overlay，`check` 同时比较 overlay 内容。`host/` 和 CLI 启动桥打包进 `Contents/Resources/PADHost/`。
- 工具和 Xcode DerivedData 在项目缓存内；Zig 的依赖/对象缓存仍使用其默认用户缓存目录。
- 仅改 Swift/工作台且已有匹配 GhosttyKit 时，可用 `build --native-only` 增量构建，跳过 Zig。仍须先 `prepare`，并移开已有输出 App。首次构建或改终端核心不得使用此选项。
- 已有源码不做强制 reset；已有输出 App 不覆盖，重建前请自行移到别处。构建是开发流程，不承诺字节级可重现。

### 已遇到的环境问题

**Xcode 26.5 SDK / Zig 0.15.2 链接错误**：该 SDK 的 `libSystem.tbd` 仅列 arm64e，固定 Zig 无法解析所需 arm64 系统符号。本机另有可用的 Command Line Tools macOS 26.2 SDK：

```sh
PAD_ZIG_MACOS_SDK=/Library/Developer/CommandLineTools/SDKs/MacOSX26.2.sdk \
PAD_ZIG_LIBTOOL=/Library/Developer/CommandLineTools/usr/bin/libtool \
  /usr/bin/python3 -u apps/pad-terminal/scripts/upstream.py build
```

SDK 设置只覆写 Zig 的 macOS SDK 路径查询；iOS SDK 查询、Metal 和原生 Swift App 仍使用选定 Xcode。不修改系统 SDK、`xcode-select` 或终端核心。不能把整个 `DEVELOPER_DIR` 切到 Command Line Tools：上游配置阶段也需要查找 iOS SDK。

**Xcode 26.6 归档问题**：其 `libtool` 合并 Zig 静态库时会警告未对齐并丢失成员，最终缺少 `_ghostty_*` 符号。上面的 `PAD_ZIG_LIBTOOL` 仅为 Zig 合并步骤指定本机可用的 CLT 归档器；路径参与构建步骤哈希，可增量重链，不必清空缓存或重编所有源码。

**Zig 下载依赖报 400 / HttpConnectionClosing**：保存失败日志，然后只恢复其中列出的固定依赖：

```sh
/usr/bin/python3 apps/pad-terminal/scripts/dependencies.py /path/to/build.log
```

下载沿用锁定的上游 URL；GitHub git 依赖使用同一固定 commit 的归档。每份下载仍由 `zig fetch` 计算内容哈希并与上游清单对照，不改版本或关闭校验。之后增量继续原构建即可。

## 隔离与许可

- App：`out/PAD Terminal Preview.app`，bundle ID `cn.ghostcloud.pad.terminal.preview`，本地 ad-hoc 签名；不安装、不公证、不发布。
- 配置：`~/Library/Application Support/cn.ghostcloud.pad.terminal.preview/config.ghostty`，以及 `${XDG_CONFIG_HOME:-~/.config}/pad-terminal-preview/config.ghostty`（兼容同目录 `config`）。不加载原有 Ghostty 配置。
- Sparkle 更新入口关闭，不下载上游更新来替换预览 App。界面部分名称/图标仍是 Ghostty，属于开发预览，并非官方 Ghostty 发布物。
- `Contents/Resources/PAD-Licenses/` 保留 Ghostty MIT、上游/已下载依赖/Swift 包的许可和声明；嵌入框架原有声明保留。
- 不覆盖 `/Applications/Ghostty.app`、`PAD Desktop.app`，不触碰 Desktop/iOS 正式数据。

## UI 基线

只复刻 cmux 的视觉与布局，不引入它的源码/品牌/功能架构。PAD 仍有自身的任务、隔离账号和原生 Pi RPC 对话，不把 Agent 换回 TUI。参考与规格见 [`native/DESIGN.md`](native/DESIGN.md)。

- 34pt 一体化顶栏、真实系统交通灯、280pt 任务侧栏；保留 cmux 布局，但不复制过小的视觉密度。
- 对话正文/终端默认 14pt；顶栏「aA」可将界面/对话切为 14/16/18pt，设置本地保存。终端独立用 ⌘+ / ⌘− 调整，不重建会话。
- 登录页固定搜索框，支持名称/ID 和 ChatGPT/Claude/Gemini 别名，已登录优先、常用入口及“已登录”筛选。搜索不触发认证。
- 原生会话采用紧凑日志式排版；模型/账号/发送/停止收在输入区状态行。
- 顶栏可隐藏侧栏/查看真实任务状态；终端 tab strip 提供右分屏、下分屏和项目终端入口。隐藏侧栏不重建 PTY。
- `PADDefaults.ghostty` 只提供终端背景、前景、字号和边距默认值，先于用户配置加载；不覆盖现有配置或修改渲染内核。

## 本地会话

侧栏「本地会话 · Codex / Claude / Pi」读取本机标准目录，无需先在 PAD 登录：

- Codex：`~/.codex/sessions/`；Claude Code：`~/.claude/projects/`（不列 subagents）；Pi：`~/.pi/agent/sessions/`。
- 搜索标题/路径/ID，按工具或当前项目筛选；面板打开且 App 活跃时每 15 秒刷新，关闭停止定时同步。只读本地文件，不上传、不读凭据。
- **在原工具中继续**：确认后在新终端窗口运行对应 CLI 的 resume，沿用该工具自己的会话/账号/配置。请先停止其他终端中的同一会话；PAD 不提供外部 CLI 的写入互斥或退出后保活。
- **Pi 导入原生面板**：完整会话树/压缩状态复制到 PAD 默认账号的私有目录，不改源文件、不复制凭据。副本并非双向同步，需显式登录/选模型才可继续。
- Codex/Claude 不转换成 Pi，也不声称原生面板已接入它们的执行后端。历史预览只显示文本/工具摘要，图片保留在原工具。
- 当前只发现标准目录；不扫描自定义数据根或 Codex 归档目录。扫描上限 5000 文件/30000 目录项，列表最多 2000 条/4 MiB，历史最多 200 条/8 MiB，截断会提示；Pi 副本上限 32 MiB。

不打断已运行预览的构建方式：`upstream.py build --native-only --output-dir out/usability`（路径相对本目录，仍禁止覆盖）。当前产物为 `out/upstream-models/PAD Terminal Preview.app`（原生增量构建）；正常退出旧 App 后打开新版。已完成的数据根迁移不需重做；只有仍使用旧根时才按下方条件显式迁移。`out/usability/` 是历史产物，不要在迁移后继续使用。

## 原生工作台

需要现有 Node.js ≥22.19 与 Pi 1.0.2（本机验证 Node 26.3.0）。App 自带桥接代码，但本轮不内置 Node/Pi，不自动下载。默认查找 Homebrew / `/usr/local` 全局 Pi；其他位置可显式指定 `PAD_NODE_PATH` / `PAD_PI_PACKAGE`。

1. 打开 App，点「选择目录…」加入项目，然后「新建任务…」。
2. 「账号与登录」只显示一个默认账号，不再创建/切换 profile；手动发起受支持的 API Key 或 OAuth 登录，授权链接只在点击后打开，认证可取消。历史任务仍使用其原 profile。
3. 为任务选模型，输入多行消息并点击「发送」。普通 Return 只换行；可点击「停止」，查看流式文本、工具状态及任务历史。
4. 任务切换不往已有 shell 注入 `cd`。「项目终端」显式创建该目录的新 Ghostty 标签页；原生分屏快捷键仍保留。Agent 与终端的分隔条可拖动调整宽度，终端内部继续支持横/纵分屏。

架构：**SwiftUI → 应用级 Node JSONL host → 每任务 Pi RPC**；账号登录/目录由同一 host 调 Pi SDK，只有用户点击发送才提交模型 prompt。共享接口见 [`host/PROTOCOL.md`](host/PROTOCOL.md)。

- 独立元数据：`~/.pad/preview/v1/workbench.json`；profile 仍使用各自的 `pi-agent/` / `pi-sessions/`。原子写入、私有权限及单宿主锁；首次创建隔离默认账号，不捡取 CLI 默认账号。
- 新任务/Pi 导入使用默认账号；历史任务固定原 workspace/profile，登录/模型目录按该任务的有效 profile 获取，不重绑任务。已有多余 profile（包括无任务的）保留在磁盘但不进入正常账号 UI。会话路径按任务固定；浏览历史不启动常驻 Pi 子进程。
- 目录读取离线，未登录时模型列表为空。不得自动切换模型/provider；登录/退出会回收该 profile 的空闲 Pi，下一次重读账号状态。
- GUI 退出通过 stdin EOF 关闭自建子进程；不提供脱离 App 的 daemon/保活。认证仍由 Pi SDK 保存于预览 profile，本轮不是 Keychain 方案。
- 开发/测试可显式覆盖 `PAD_TERMINAL_DATA_ROOT`；不设置时使用上面的独立预览目录。`PAD_TERMINAL_HOST` 仅作宿主脚本开发覆盖。

### OpenAI 账号模型目录

- OpenAI/ChatGPT OAuth 使用 [官方 SIWC 账号目录](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference/)：仅点击「同步 OpenAI 模型」才经 SDK 解析 OAuth bearer，发起 `GET https://api.openai.com/v1/models`。启动、打开菜单、切换任务/账号不联网；不回退到静态 OpenAI OAuth 模型表。
- 解析官方 `models[]`（不是通用 API 的 `data[]`），仅展示 `visibility: "list"`，保留上游顺序，以 `display_name` 展示名称、`slug` 作为模型 ID，按 slug 去重。API Key 和其他 provider 仍用 Pi SDK 模型目录，语义不同。
- 每个 profile 的 `pi-agent/pad-openai-models.json` 是原子写入的 0600 私有离线元数据缓存，不保存 bearer。UI 区分官方账号目录 / Pi SDK 来源，以及未加载、fresh、stale、error；重启读取为 stale，同一凭据绑定同步失败可保留 stale 列表。列表不保证推理权限或成功；runtime 不支持的 slug 可见但禁用，host 也拒绝选择。
- 凭据文件元数据变化会丢弃旧缓存；SDK token 轮换或并发凭据变化可能要求再显式同步一次。认证解析期间变更会提示「凭据已更新，请再次同步模型」，本次不请求/发布目录。不解析 JWT、不读原始 token 来推断身份。
- 已有 OpenAI 登录保留，无需迁移或重新登录；不静默改选模型或任务账号。**推理强度（reasoning-strength）控制明确延期，尚未实现。** 本轮未验证真实认证端点、账号模型数量或推理请求。

下一步：正常退出旧 App，在已配置好代理环境的终端直接运行 `apps/pad-terminal/out/upstream-models/PAD Terminal Preview.app/Contents/MacOS/ghostty`，再显式点击「同步 OpenAI 模型」。不要把真实代理值/凭据写进命令参数或日志。

### 数据根与显式迁移

默认根为 `~/.pad/preview`，非空 `PAD_TERMINAL_DATA_ROOT` 显式覆盖仍保留。旧根是 `~/Library/Application Support/PAD Terminal Preview`。仅旧根存在时新版启动会**阻止初始化**并给出迁移/显式选择根的提示，不会静默创建空账号，也不会自动迁移。两个根同时存在且没有有效迁移回执（或旧源随后改变）时也拒绝默认启动，需人工确认根，绝不合并。

先正常退出所有使用该 PAD 数据根的 PAD App、host、独立 `pi-session.mjs` 客户端（无需关闭无关的本地 Pi/Codex 客户端），再执行以下命令；仅在旧根存在、新根不存在时适用：

```sh
cd /Users/tim/tools/pad
# 先手动关闭上述客户端；取消为旧根进行中的认证。
env -u PAD_TERMINAL_DATA_ROOT node apps/pad-terminal/scripts/migrate-data-root.mjs --confirm-closed
open "apps/pad-terminal/out/auth-diagnostics/PAD Terminal Preview.app"
```

App 内也包含入口：`Contents/Resources/PADHost/scripts/migrate-data-root.mjs`，使用现有 Node 运行。迁移私有暂存、排他创建目标、最后写回执并清除 incomplete 标记；拒绝已有目标、锁、符号链接/特殊文件和变化中的源，不覆盖/合并。失败时保留需调查的未完成数据，勿绕过锁或标记猜测恢复。源目录保留，任务路径/结构会重定位，但凭据与正文不改写。**迁移后不要继续运行旧安装写入保留源**，否则两根会分叉、默认启动被拒绝。未对用户 home 执行迁移。

### 认证边界与 UUID 修复

- 仅展示安装的 Pi SDK 中有可调用 `login` 的方法，并在 host 执行同一策略；模型目录也过滤禁止的凭据，不自动替换 provider/model。
- PAD 原生登录排除 Claude 订阅 OAuth 和 Gemini CLI/Antigravity 订阅 OAuth；保留 Anthropic/Gemini API Key 路径和 SDK 支持的其他方法/已配置云模型。依据 [Claude 认证与凭据使用](https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use) 与 [Gemini CLI 条款/隐私](https://geminicli.com/docs/resources/tos-privacy/)。不声称已审计所有其他 provider 政策；独立 Pi TUI 不由此原生策略管理。
- OpenAI/ChatGPT 登录原先缺少安装 UUID 导致的 `Authentication failed` 已离线修复：SDK 接收懒加载同步 device ID 回调，`v1/installation-id.json` 每个 PAD 根稳定保存，迁移原样保留。损坏身份不会自动旋转/覆盖；不是 OpenAI API Key 或机器全局 ID。
- 认证提示保留授权链接，错误仅输出安全分类；凭据已保存但刷新失败会明确提示不要重复登录。登录/退出等待空闲任务子进程回收。真实 OAuth/API Key 登录与模型回复仍未验证。

### HTTP 初始化与代理

host 在加载 Pi SDK 前，从已安装 Pi 包加载公共 Undici API，安装 fetch 与全局 dispatcher；不导入 Pi 私有 HTTP 实现，也不读取全局 Pi `httpProxy` 配置，不能声称与用户 Pi 设置使用相同网络路由。

- 原生 host 显式转发 HTTP/HTTPS/ALL/NO_PROXY 的大小写环境变量；同项小写优先，空字符串也是显式设置，禁止系统代理自动补充。HTTPS 未设置时继承 HTTP；显式空 HTTPS 直连。ALL 仅在 HTTP/HTTPS 都未设置时生效，NO_PROXY（包括空值）保留。
- 无显式代理时仅有限支持 macOS 静态 HTTP CONNECT / SOCKS5；不读取代理凭据。PAC/自动发现或不兼容的 bypass/简单主机排除可能阻止采用系统代理，UI 会警告；不实现系统路由等价转换。
- Finder / `open` 通常不继承当前终端的代理环境。需要环境代理时，在已适当配置环境的终端直接运行新 bundle 的 `Contents/MacOS/ghostty`；不要把真实代理地址或凭据写进文档、命令参数或诊断日志。

### 安全认证诊断

仅用户新发起的 OpenAI OAuth 尝试尽力写入 `<root>/v1/auth-diagnostics.jsonl`（默认 `~/.pad/preview/v1/auth-diagnostics.jsonl`）。只保留 timestamp、providerId、method、白名单 phase/category/outcome，以及可证明的 HTTP status / 白名单 errno；不保存授权 URL、code/token、SDK 原文、账号或路径。最多 100 条 / 64 KiB，目录 0700、文件 0600，拒绝符号链接；写入失败不阻断登录，启动失败可能无记录。

新 token 分类只识别标准 OAuth 错误码（invalid_request/client/grant、unauthorized_client、unsupported_grant_type、invalid_scope）或有界响应体形状（HTML、未识别 JSON / body）；不保存原始 body/URL/token，不证明 403 的原因、代理来源或授权码过期/复用。阶段可能为 `unknown`；观察到的提示不证明后续步骤完成，也不能证明网络原因。之前的 403 无法重建，不能恢复历史诊断。请正常退出当前 PAD，再打开新诊断产物并发起一次**全新 OpenAI OAuth**；旧 callback 链接不能诊断或重试上次失败。本更新增加安全诊断，不是对未知 token exchange / 凭据存储失败的已证实修复；真实 OAuth 仍未验证。

## 可选 Pi TUI 入口

需要现有 Node.js ≥22.19 与 Pi CLI（已验证 Node 26.3.0 / `@earendil-works/pi-coding-agent` 1.0.2）。在预览终端某个分屏运行：

```sh
cd /path/to/pad
node apps/pad-terminal/scripts/pi-session.mjs --profile default
# 或为一个开发任务选择稳定 ID：
node apps/pad-terminal/scripts/pi-session.mjs --profile default -- --session-id terminal-demo
```

默认寻找 Homebrew / `/usr/local` 全局 Pi 包；其他安装位置用 `PAD_PI_PACKAGE=/absolute/package/path`，不会自动安装或升级 Pi。

- 每个 profile 独立保存到 `~/.pad/preview/v1/profiles/<profile>/`，分别使用 `pi-agent/` 和 `pi-sessions/`。
- 复用 PAD 的 profile → agent/session 目录、环境白名单和 Pi 会话 ID 思路；不自动复制 Desktop 账号/数据库/历史，不导入 Codex/ChatGPT 凭据。本地会话的显式只读接入与 Pi 副本导入见上方。
- 默认离线启动，禁用遥测、版本检查、自动加载扩展/技能/提示模板/主题；项目 `AGENTS.md` 仍是正常项目上下文。不传入环境中的模型 Key、`NODE_OPTIONS` 或当前 Pi 会话变量，没有默认 prompt。
- 需要真实模型时，由使用者在此独立 profile 内自行 `/login`、`/model`；离线启动开关不是网络沙箱，之后主动提问仍可能产生费用。普通 shell 和 Pi 工具仍具有当前 macOS 用户权限。
- 这只是可选的 **原生终端 PTY → Node → Pi TUI** 调试路径，与上方原生 Agent 面板不同；不需要 Electron Utility Process。
- 独立持久会话服务、完整 tmux 替代、Desktop/iOS 迁移不在本轮范围。

## 已验证与待验收

原终端阶段已验证原生分屏、PTY 尺寸、中文文本和隔离 Pi TUI。工作台阶段增加离线 host 主路径、Swift transport/流式事件夹具及原生增量构建；真实登录与付费回复未测试。详见 [VALIDATION.md](VALIDATION.md)。

此前无活动显示器时仅临时关闭 VSync；当前会话已恢复活动显示器，正常启动无需此参数。仅相同无头环境可使用：

```sh
LLVM_PROFILE_FILE="$PWD/apps/pad-terminal/.cache/preview-%p.profraw" \
GHOSTTY_MAC_LAUNCH_SOURCE=cli \
  "apps/pad-terminal/out/PAD Terminal Preview.app/Contents/MacOS/ghostty" \
  --config-default-files=false --window-vsync=false --window-save-state=never
```

若命令来自后台 launchd 会话，需在已登录用户的 Aqua 会话运行，例如 `launchctl asuser "$(id -u)" /usr/bin/env GHOSTTY_MAC_LAUNCH_SOURCE=cli /absolute/path/to/ghostty ...`。无需修改系统权限。无头运行不能替代真实显示器验收。本机 ReleaseLocal 会产生 LLVM profiling 文件，上例将其限定到开发缓存；本轮不作性能基准结论。

人工待确认：中文 IME、画面流畅度/混排、字号、拖拽、剪贴板、Vim，以及真实账号登录→选模型→回复→停止→重启恢复。已验证元数据/离线历史夹具恢复，但不是付费会话全链验收。未改动用户剪贴板，不做全量 E2E、压力测试、正式发布或 GUI 退出后保活承诺。

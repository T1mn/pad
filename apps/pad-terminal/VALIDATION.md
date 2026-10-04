# 开发验证记录

2026-10-04，本机 Apple Silicon / macOS 26.2；不是正式发布验收。

## 官方 OpenAI 账号模型目录（2026-10-04，当前未发布预览）

- 新产物：`out/upstream-models/PAD Terminal Preview.app`；未覆盖旧 App、启动 GUI、改变用户进程或迁移数据。已有 OpenAI 登录保留，无需重迁移/重登录；不静默改选模型或任务账号。
- 唯一审查修复：SDK `getAuth` 期间凭据文件绑定变化不再接受旧 bearer 配新元数据；删除旧缓存并提示「凭据已更新，请再次同步模型」，本次不 fetch/发布。新增一条 fake 回归覆盖已有缓存、bearer 解析后更换元数据、无额外 fetch/无缓存发布。
- 指定 `node --test apps/pad-terminal/host/openai-model-catalog.test.mjs` 一次运行：6 passed / 0 failed / 0 skipped，退出 0；无实际检查失败/重跑。仅 fake SDK/fetch/临时文件，日志 `.cache/logs/upstream-models-tests.log`。
- 一次 `upstream.py prepare` 和一次 `build --native-only --output-dir out/upstream-models` 均退出 0，`BUILD SUCCEEDED`；日志 `.cache/logs/{prepare-upstream-models,build-upstream-models}.log`。复用 GhosttyKit，无 Zig/full Swift typecheck。上游原生编译存在 unused result、Sendable/actor 与调试符号警告，未阻断编译/签名。
- 产物检查：bundle ID、所有非测试 host / 两个 CLI 字节一致、strict deep codesign、`upstream.py check`、scoped whitespace（排除 patch/cache/reference）记录在 `.cache/logs/artifact-upstream-models.log`。
- [官方 SIWC](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference/) `GET https://api.openai.com/v1/models` 使用账号 `models[]`：仅 `visibility:"list"`，上游顺序、`display_name` 名称、`slug` ID；不是通用 `data[]`。仅显式「同步 OpenAI 模型」联网，启动/菜单离线，无静态 OpenAI OAuth fallback；API Key/其他 provider 仍用 SDK 目录。
- 私有 0600 profile 缓存只存元数据，重启为 stale；同绑定同步失败可显示 stale。UI 区分官方账号/SDK 来源与未加载/fresh/stale/error；runtime 不支持 slug 禁用，目录不是推理权限/成功保证。保守文件元数据绑定会因 token 轮换/并发更改失效，可能需再显式同步；不解码 JWT/读取原始 token 判断身份。
- Swift DTO 可选字段编译已验证，JSON roundtrip 未执行。现有 `scripts/native-smoke.swift` 可在 isolated fake host 夹具下扩展旧/新 catalog JSON decode/encode 断言，但当前 harness 依赖 transport/root；没有为此额外运行或修改它。推理强度控制明确延期/未实现。
- 未调用真实凭据、账号端点、登录或模型；不声称端点成功/真实模型数量。未重跑已通过的 auth/proxy/root 检查，无全量套件/E2E。下一步用户正常退出旧 App，在已有代理环境的终端直接启动新 bundle `Contents/MacOS/ghostty`，显式点击「同步 OpenAI 模型」。

## HTTP / 安全 token 分类集成（2026-10-04，历史未发布预览）

- 新产物：`out/login-transport/PAD Terminal Preview.app`；未覆盖旧 App、启动 GUI、退出用户进程或迁移数据根；已迁移状态无需重迁移。
- 指定 `http-client.test.mjs` + `auth-diagnostics.test.mjs` 一次运行：7 passed / 0 failed / 0 skipped，退出 0，无修复/重跑。真实 Undici 回归使用禁止联网的 MockAgent；其余为合成 SDK/错误/临时根。日志 `.cache/logs/login-transport-tests.log`。
- 一次 `upstream.py prepare` 和一次 `build --native-only --output-dir out/login-transport` 均退出 0；日志 `.cache/logs/{prepare-login-transport,build-login-transport}.log`。复用 GhosttyKit，无 Zig 重编、额外 Swift 全量 typecheck 或全量测试。
- 产物检查记录见 `.cache/logs/artifact-login-transport.log`（bundle ID、非测试 host / 两个 CLI 字节、strict deep 签名、upstream source check 与 scoped whitespace）。
- host 在 SDK 前使用已安装 Pi 的公共 Undici 初始化 HTTP；显式代理环境小写优先、空值保留，未设置 HTTPS 才继承 HTTP，ALL 仅用于两个协议都未设置。PAD 不读取全局 Pi `httpProxy`，未证明与用户 Pi 设置路由相同。系统代理只有限支持静态路由；PAC/自动发现或不兼容 bypass 可阻止采用，UI 会警告。Finder / `open` 通常不继承终端代理环境；需要时从配置好环境的终端直接运行 bundle `Contents/MacOS/ghostty`，勿把真实代理值/凭据放入文档或命令参数。
- 新安全 token 类别为标准 OAuth code 或有界 body 形状，不证明 403 原因；无原始 body/URL/token 留存，无法重建之前的 403。未读真实诊断/凭据/会话、未探测网络、未验证真实 OAuth 或模型回复，不声称 403 已修复。下一步由用户正常退出旧 App→打开新产物→发起全新登录，勿重放旧 callback；不需重做已完成迁移。

## 安全认证诊断（2026-10-04，历史诊断预览）

- 当前产物：`out/auth-diagnostics/PAD Terminal Preview.app`；复制 `out/account-fix/` App 并保留 bundle/符号链接，通过已检查的 `upstream.py bundle_host` 同步资源。无 Swift/native 改动，无 Xcode/Zig/native 构建；旧 App/用户进程未修改、未启动或退出应用。
- `auth-diagnostics.mjs` 仅对显式 OpenAI OAuth 尽力记录 `<root>/v1/auth-diagnostics.jsonl`：白名单 timestamp/providerId/method/phase/category/outcome、可证明的 HTTP status / 白名单 errno；无 URL/code/token、SDK 原文、账号或路径。上限 100 条 / 64 KiB，目录 0700 / 文件 0600，拒绝符号链接；可能 phase unknown，启动失败可能无日志，不恢复历史失败。
- 仅修正测试临时根为 `realpath(os.tmpdir())`，保留生产 writer 及显式符号链接拒绝断言。指定 `node --test apps/pad-terminal/host/auth-diagnostics.test.mjs` 一次通过：2 passed / 0 failed / 0 skipped，退出 0，无重跑；仅合成离线夹具。日志 `.cache/logs/auth-diagnostics-tests.log` 含实际退出码。
- 复制/资源同步、保留 entitlements/flags/runtime 的 ad-hoc 重签、`codesign --verify --deep --strict` 均退出 0。15 个当前非测试 host 模块 + 两个 CLI 逐字匹配，bundle ID 仍为 `cn.ghostcloud.pad.terminal.preview`；证据 `.cache/logs/artifact-auth-diagnostics.log`。仅本次修改的 source/docs 做一次 scoped whitespace 检查（排除 patch）。
- 未读真实诊断/凭据/用户 OAuth code，未运行其他套件、GUI、网络、登录或模型请求；未重复历史迁移/native 验证。请正常退出当前 PAD→打开此新产物→发起一次全新 OpenAI OAuth；旧 callback 链接不能诊断/重试旧失败。这是安全诊断增量，非未知 token exchange / 存储失败的已证实修复，真实 OAuth 仍未验证，网络原因未证实。

## 账号 / 数据根集成（2026-10-04，历史 account-fix 预览）

- 当前新产物：`out/account-fix/PAD Terminal Preview.app`；旧产物/运行进程未覆盖或关闭。默认根改为 `~/.pad/preview`，仅旧根存在会按设计阻止初始化，需用户关闭使用 PAD 数据根的客户端并显式迁移（命令见 README），或显式覆盖根。本轮未检查/迁移用户 home 数据。
- 原生 UI 仅一个默认账号；新建任务/Pi 导入使用默认 profile，历史任务保留其有效 profile，额外历史 profile（含无任务的）不删除但不进入正常 UI。SDK 可调用登录方法 + PAD 订阅 OAuth 排除策略已实现，保留 API keys；其他 provider 政策并未全部审计。
- OpenAI/ChatGPT `Authentication failed` 的缺少 UUID 原因已离线修复：每根稳定身份、lazy 同步 SDK callback、迁移原样保留；安全错误区分身份/SDK/回调端口/凭据已保存但刷新失败，授权 URL 保留，认证结束等待空闲 peer 清理。不代表真实 OAuth 已通过。
- 一次 `node --test` 运行指定的 `host-env`、`data-root-migration`、`installation-id`、`pi-sdk`、`workbench-auth` 五个套件：17 passed / 0 failed / 0 skipped，退出 0，无初始失败/重跑。全部 fake SDK/临时根，未加载真实凭据或请求模型。日志 `.cache/logs/account-fix-node-tests.log`。
- `upstream.py prepare` 退出 0（`.cache/logs/prepare-account-fix.log`）；一次 `build --native-only --output-dir out/account-fix` 退出 0、`BUILD SUCCEEDED`（`.cache/logs/build-account-fix.log`，退出码另存 `build-account-fix.exit`）。复用 GhosttyKit，无 Zig 重编；原生编译兼作类型验证，无额外 swiftc 全量检查。打包入口新增 migration CLI，不改核心补丁。
- 新产物 bundle ID `cn.ghostcloud.pad.terminal.preview` 核对、14 个非测试 host 模块 + 两个 CLI 与源码逐字相同、严格 deep codesign verify、`upstream.py check` 与 tracked scoped `git diff --check` 均退出 0（`.cache/logs/artifact-account-fix.log`）。首次检查包装器误将 untracked `git diff --no-index --check` 的“存在差异”退出 1 当作失败（无 whitespace 输出），整体命令退出 1；仅修正该检查器并重跑 whitespace 部分，记录 `.cache/logs/whitespace-account-fix.log`。未重复已通过产物检查/构建，不复用历史证据。
- 未运行 GUI、真实认证/回复、真实数据迁移、全量套件/E2E/压力测试。用户仍需手动验收关闭旧根客户端→迁移→打开新 App，以及真实受支持登录/选模型/发送/停止/恢复。Node/Pi 是外部依赖，App 为本地 ad-hoc 未公证/未发布预览。Desktop/iOS 未提交改动保持不动。

## 可用性与本地会话增量（同日，历史 usability 产物）

- 新产物在 `out/usability/PAD Terminal Preview.app`；旧 `out/` 实例未关闭/覆盖，用户退出旧实例后再打开新版。构建增加受仓库目录约束的 `--output-dir`，不用停掉正在用的 App。
- 主要界面字号增大，对话/终端默认 14pt；原生字号菜单为正文 14/16/18pt。账号页固定搜索、别名、常用 Provider 和已登录筛选；Return 搜索不关闭账号页、不触发登录。
- 本地会话标准目录的只读发现、刷新/搜索、历史预览、原 CLI 续接准备、Pi 私有副本导入已实现。Codex/Claude 不伪装成 Pi 会话；没有双向同步，也未建立它们的原生执行后端。
- 本机只读扫描发现 Codex 878、Claude 30、Pi 47 条，约 963ms（一次开发观察，不是性能基准）；没有把真实对话内容写进验证日志，没有读凭据或调用模型。
- Swift 类型检查/原生增量编译通过；离线回归覆盖三种格式、Pi 分支、Codex 去重、文件更新、路径/符号链接拒绝、完整 Pi 副本与私有权限、搜索别名/筛选、DTO 和 shell argv 引用。没有启动本地真实 AI CLI、登录或付费请求。
- 本地会话回归此前因 `/var` 与 `/private/var` 夹具预期不一致失败；修正为真实路径后定向重跑通过（2 passed / 0 failed），验证 cwd 规范化与续接比较。当前 11 个非测试 host 模块已逐字同步到 usability App，bundle ID 核对、保留 entitlements/flags/runtime 的 ad-hoc 重签及严格嵌套签名验证通过；本次未重编原生或重跑 GUI。
- 后续有界修复：缓存内部保留原始有界 cwd（不进入公开 DTO），会话文件指纹未变时重新规范化原始路径，避免缺失项目/符号链接目标恢复后永久无法续接；resume 仍重读并核对源标识、原始 cwd 与规范路径。新增一条离线恢复夹具，定向 `node --test apps/pad-terminal/host/local-sessions.test.mjs` 实际通过（3 passed / 0 failed / 0 skipped，退出 0）；日志 `.cache/logs/local-sessions-restoration-tests.txt`。通过已检查的 `bundle_host` 同步 11 个非测试 host 模块，host/入口/默认配置逐字一致；保留 entitlements/flags/runtime 的 ad-hoc 重签及 `--verify --deep --strict` 均退出 0。本次不重编 Swift/Zig、不重复上游/native 检查、不操作 GUI/真实凭据或会话、不启动真实 CLI/登录/模型请求；真实续接与认证限制不变。
- 用独立 HOME/数据根的夹具实例取得主窗口截图，确认新字号与本地会话入口绘制。辅助功能未枚举出该测试实例的窗口（旧实例并存），账号搜索/字号菜单/导入的完整 GUI 交互验证未完成；测试实例已关闭，旧实例保留。最后一次文字检查增量不重编 Zig。
- 此历史阶段的 `Authentication failed` 仍待确认实际 OAuth/API Key 失败步骤，搜索改进不是认证故障修复；当前离线 UUID 修复证据见顶部，真实 OAuth 仍未验证。真实会话在原 CLI 中的续接仍需用户点击验收，不自动执行。

证据：`.cache/logs/build-usability.log`、`local-sessions-tests.txt`、`readability-smoke.txt`、`usability-window.png`、`usability-gui.log`。未做全量/E2E/压力测试、正式发布或 Desktop/iOS 迁移。

## cmux 风格 UI 增量（同日，历史产物）

- 依据公开截图独立实现：30px 一体顶栏/原生交通灯、248px 蓝色纵向任务标签、左右 Agent/终端、窄 tab strip、等宽日志流和紧凑输入区。不引入 cmux GPL 源码/品牌/架构；业务状态和 Pi RPC 未改。
- 界面类型检查及原生增量编译通过。截图检查发现标题栏重复留白、原生底色遮挡控件，局部修正 AppKit 装饰后增量更新；未重编 Zig、未重跑后端/Desktop/iOS 全套。
- 定向 UI 主路径通过：顶栏距窗口顶 3px；隐藏/显示侧栏不重启 PTY/host；下分屏按钮创建真实 Ghostty split；账号 sheet 的 API Key/OAuth 入口可见，关闭正常；没有发起登录或模型请求。
- 最终截图确认顶栏控件已可见。最后一次鼠标点按验证遇到系统自动锁屏（`CGSSessionScreenIsLocked=1`、前台 `loginwindow`），未完成，不能据此声称最终鼠标交互全通过；未尝试解锁或改变 TCC。
- `PADDefaults.ghostty` 已打包并实际加载；固定源码/补丁/overlay 一致，bundle ID、host 与视觉资源逐字核对、严格嵌套签名检查均通过。上游现为 12 个文件的薄补丁，新增范围仅原生窗口包装/标题栏和视觉配置入口，不改终端解析/渲染核心。
- 当前预览保持打开，仍用隔离 smoke 数据；截图中的历史明确标为离线夹具，不是真实模型回答。此前 Desktop/iOS 改动保留，未发布/打 tag/推送。

证据：`.cache/logs/build-cmux-ui.log`、`cmux-ui-smoke.txt`、`cmux-ui-window.png`、`cmux-ui-gui.log`、`workbench-gui.pid`。最终指针验收需解锁后补做。

## 原生工作台增量（同日，cmux UI 之前）

- 已实现并打包 SwiftUI 项目/任务侧栏、原生 Agent 面板、账号/API Key/OAuth 交互、模型选择与底部 Ghostty 终端。入口不再是 Pi TUI；原生层通过独立 Node host 与 Pi RPC 通信。
- 集成编译曾暴露 escaping closure 的显式 `self` 问题，修正后增量成功；随后只针对审查修正增量更新 Swift/资源，未 clean 或重编 Zig 内核。最终日志 `.cache/logs/build-workbench-final.log`，签名及许可打包通过。
- 共用 Swift DTO/界面类型检查、少量 fake Pi/临时目录回归通过；覆盖账号互斥/取消、发送启动中 abort、背压不误判失败、过期请求 ID 不错配。未运行 Desktop/iOS 全套检查。
- 真 Pi 离线主路径：新建 workspace/两个隔离 profile/任务 → 42 个静态 provider、0 个可用模型 → 无模型选择的发送被拒绝 → 真实 RPC 本地 `bash` 输出 `PAD_NATIVE_HOST_RPC_OK 中文` → stats token/cost 均 0 → 关闭自建 Pi。
- 元数据重启恢复、标明“非模型会话”的历史夹具恢复通过；离线浏览历史不创建常驻 Pi。不是付费模型会话恢复的端到端证明。
- Swift transport 已实际启动 Node host，解码 snapshot/catalog/history 并正常 shutdown；reducer 夹具覆盖 Unicode 分隔字符、多内容块、authoritative message_end、toolCallId 与 settled。夹具不是模型回答。
- 活动显示器为 1；新窗口标题 **PAD Native Workbench**。定向辅助功能操作选中任务并显示离线历史，打开账号 sheet 可见 API Key/OAuth 入口；未点击任何登录按钮。工作台内 `⌘D` 保留两个真实 PTY。
- 已取得此预览窗口截图，确认三区布局/中文文案实际绘制。测试窗口退出后，该窗口对应 App、Node host、两个 login 子进程均消失；随后重开最终产物供体验。此前终端-only 演示会话未关闭。
- 此阶段上游差异为 9 个已有文件；工作台源文件作为独立受校验 overlay，不改终端解析/渲染核心。未安装覆盖、提交、打 tag 或推送。

新增证据（开发缓存）：
- `.cache/logs/workbench-smoke-result.json`：独立数据路径、42 provider、零 token/cost、恢复与退出结果。
- `.cache/logs/native-smoke-result.txt`：`PAD_NATIVE_TRANSPORT_AND_REDUCER_OK`。
- `.cache/logs/workbench-ui-account.txt`、`workbench-window.png`：原生任务历史/账号入口/分屏证据。
- `.cache/logs/workbench-gui.log`：此阶段演示启动记录；使用 smoke 独立数据，不是正式账号。当前记录见上方 cmux UI 段落。

按改动选择一条轻量检查，不要求每次全部执行：
```sh
/usr/bin/python3 apps/pad-terminal/scripts/upstream.py check
node apps/pad-terminal/scripts/workbench-smoke.mjs
```
`native-smoke.swift` 的编译/运行说明在文件顶部；使用上一步新建的数据根及显式 `PAD_TERMINAL_HOST`。不要对用户正式数据运行 smoke。

## 原终端阶段通过记录

- 固定 Ghostty v1.3.1 commit 与补丁完全一致；Zig 0.15.2、Nushell 0.116.0、Xcode 26.6、Metal 工具可用。
- 原生开发编译完成。SDK/下载问题在配置阶段修正；首次链接暴露归档器问题后仅做增量重链，没有重复 clean 全量构建。
- `out/PAD Terminal Preview.app` 的 bundle ID 为 `cn.ghostcloud.pad.terminal.preview`；上游 `+version` 为 1.3.1、CoreText/Metal。App 自身开发版显示版本为 0.1。
- ad-hoc 签名与嵌套签名验证通过；63 份许可/声明随包保留，其中 Ghostty MIT 与固定源码逐字一致。
- 通过已授权辅助功能向**本次预览进程**发送 `⌘D`、`⌘⇧D`，得到三个 PTY（`ttys010`、`ttys011`、`ttys012`），不是三个独立 App。
- 普通 zsh 接收键盘文本并写出 `PAD_NATIVE_SHELL_OK 中文`；这验证 UTF-8 文本路径，不代表中文输入法验收。
- 重载临时配置后执行窗口尺寸动作，同一 shell 的 `stty size` 从 `35 49` 变为 `45 75`，原生窗口报告尺寸 `1184×789`。
- 第三个面板运行隔离 Pi 1.0.2 TUI（Node 26.3.0），另两个面板保留 shell；本地 `!printf` 写出 `PAD_PI_TUI_OK`，没有发送模型 prompt。
- 独立新目录下的 RPC smoke：`get_state` → 本地 `bash` → `get_session_stats` → stdin EOF 正常退出；结果 `PAD_PI_RPC_OK`，token/cost 均为 0。未使用真实账号或模型 Key。
- 开发入口的两个针对性测试通过（环境/账号隔离、profile 路径拒绝）；Python/Node/shell 语法、Xcode plist、补丁一致性和 diff whitespace 检查通过。

## 本机证据（均在忽略的开发目录）

- `.cache/logs/build-native-link.log`：`BUILD SUCCEEDED`、许可收集、签名校验、最终输出路径。
- `.cache/logs/preview-headless-2.log`：三个原生终端子进程、中文字体 fallback、临时配置重载。
- `.cache/smoke-workspace/{shell-ok,third-pane,pi-tui-ok,size-before,size-after}.txt`：主路径输出。
- `.cache/pi-smoke-8So2zN/result.json`：Pi RPC 状态与零费用统计。
- smoke 时进程链：Preview `57404` → 三个 login/PTY；第三个 login `62075` → launcher `62076` → Pi `62720`。PID 只用于此次记录，不应复用来操作进程。验证后已关闭本次测试进程。
- ReleaseLocal 产生的 profiling 文件已移入 `.cache/logs/preview-default.profraw`，没有加入源码。

重复轻量检查（不构建原生 App）：

```sh
/usr/bin/python3 apps/pad-terminal/scripts/upstream.py check
node --test apps/pad-terminal/scripts/pi-session.test.mjs
node apps/pad-terminal/scripts/pi-smoke.mjs
```

## 限制与人工验收

- 原终端阶段曾无活动显示器，临时关闭 VSync 才能 smoke；本轮已恢复活动显示器，默认 VSync 可正常创建窗口。未修改渲染内核/产品默认值。
- 原 AppleScript 查询超时，改用已授权辅助功能定向操作。现已取得工作台窗口截图，但截图不能证明视觉流畅度；未修改 TCC 权限。
- 真实显示器上的 Metal 画面、中文 IME 候选框/组合文本、视觉字号缩放和混排宽度、窗口拖拽、选区/复制粘贴、Vim 交互仍待人工检查。字号快捷键已发送，但未据此宣称视觉通过；未改动用户剪贴板。
- 真实 OAuth/API Key 登录、付费回复、真实运行中的停止与付费会话恢复尚未验证，需要用户主动验收；没有后台会话服务，不承诺 GUI 退出后保活。
- 仍保留部分 Ghostty 菜单/图标；桥接代码已打包，但 Node/Pi 仍是外部依赖，尚不是独立分发产品。
- Desktop/iOS 的此前未提交修复保留，本轮没有重跑其已完成检查；没有安装覆盖、打 tag、推送、全量 E2E 或压力测试。

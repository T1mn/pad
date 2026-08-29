# desktop_runtime

- `../desktop_runtime.rs`：PAD Desktop Control Plane 的最小 facade；组合私有 Store、Pi Profile supervisor 与事件状态回写，并由 Rust 选择可信的内置 Pi launcher；自动审批只使用 Profile→Project→Task 合并后的 `permission_policy` 决策，跨 Profile 引用不会继承权限，含变量、替换、重定向或控制符而无法静态证明目标安全的 shell 命令绝不自动确认。
- `catalog.rs`：SQLite UI state、Profile、Project、Task catalog 与 Codex Sidebar snapshot；active Profile/selected Task 写前归一化，账号切换后的层级只包含该 Profile 的 Project/Task。
- `bridge.rs` / `bridge/protocol.rs`：`pad __internal desktop-server` 的有界 stdin/stdout JSONL transport 与 renderer-safe DTO；`bridge/actions.rs` 再按 `bridge/actions/account.rs`、`bridge/actions/navigation.rs`、`bridge/actions/task.rs` 拆分 v2 授权和动作处理。协议 v2 只暴露当前 Profile 的 Project/Task/Sidebar，并在每个 task、auth、PTY 控制入口重复验证 active Profile；DTO、历史、事件、工具输出与错误统一隐藏 Full Access 的全部默认保护命名空间、`CODEX_HOME` 和 Profile 私有目录，同时保留 v1 请求/轮询兼容；renderer 永远不能选择 Pi 或登录进程。
- `auth.rs`：Rust 唯一拥有的 Pi 登录/登出协调器；按 Profile 私有目录调用可信 Pi SDK，使用 attempt/prompt JSONL 状态机向 Desktop 暴露安全认证快照，不向 Electron 暴露认证路径、环境或凭据。
- `terminal.rs`：Rust 唯一拥有的单 pane PTY 桥；工作目录只从 Task Store 解析，renderer 不能传程序/cwd/env，输入、尺寸、pane 数与纯文本 snapshot 行数均有界。
- `data_root_lock.rs`：`${PAD_DESKTOP_DATA_DIR}/v1/store/desktop-server.lock` 非阻塞独占锁；一个数据根同一时间只允许一个 desktop-server 拥有 SQLite、Pi/auth 与 PTY 生命周期。
- `helpers.rs`：会话 JSONL 只读读取、Profile provider 认证摘要、路径边界和运行时状态转换等纯辅助逻辑。
- `tests.rs`、`bridge/tests.rs`、`bridge/tests/security.rs`、`bridge/tests/control_plane.rs`：DesktopRuntime/JSONL bridge 的 focused contract cases，包含跨 Profile 列举、历史、运行时、认证和 PTY 越权回归。
- Desktop UI 后续只需通过该 facade 调用 start/prompt/poll/stop，不直接操作 Pi 子进程或 SQLite。

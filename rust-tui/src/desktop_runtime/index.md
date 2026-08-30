# desktop_runtime

- `../desktop_runtime.rs`：PAD Desktop Control Plane 的最小 facade；组合私有 Store、Pi Profile supervisor 与事件状态回写，并由 Rust 选择可信的内置 Pi launcher；自动审批只使用 Profile→Project→Task 合并后的 `permission_policy` 决策，跨 Profile 引用不会继承权限，含变量、替换、重定向或控制符而无法静态证明目标安全的 shell 命令绝不自动确认。
- `catalog.rs`：SQLite UI state、Profile、Project、Task catalog 与 Codex Sidebar snapshot；active Profile/selected Task 写前归一化，账号切换后的层级只包含该 Profile 的 Project/Task。
- `model_catalog.rs`：以 Pi `ModelRuntime` 为唯一模型目录来源，按 Profile 私有 agent 根读取完整/可用模型，归一化安全元数据并隔离认证、路径和 SDK 错误。
- `bridge.rs` / `bridge/protocol.rs`：`pad __internal desktop-server` 的有界 stdin/stdout JSONL transport 与 renderer-safe DTO；`bridge/actions.rs` 再按 `bridge/actions/account.rs`、`bridge/actions/navigation.rs`、`bridge/actions/task.rs` 拆分 v2 授权和动作处理。协议 v2 只暴露当前 Profile 的 Project/Task/Sidebar，并在每个 task、auth、PTY 控制入口重复验证 active Profile；DTO、历史、事件、工具输出与错误统一隐藏 Full Access 的全部默认保护命名空间、`CODEX_HOME` 和 Profile 私有目录，同时保留 v1 请求/轮询兼容；renderer 永远不能选择 Pi 或登录进程。
- `bridge/remote_events.rs`：唯一 bridge owner 上的 Pi central pump；一次消费 stdout 后同步 fan-out 给本地 renderer 与 Profile-scoped Remote event ring，并接管 `request_pi` 保留的 deferred poll，避免历史请求吞掉交互；每个 `task_output` 都附带权威 pending UI 快照，超时清空即使没有 Pi 帧也会发布。
- `interactions.rs`：保存、校验与响应 confirm/select/input/editor 交互；placeholder 与预填值分离，取消响应显式建模，Full Access 自动回答不会留下幽灵卡片。
- `remote/` / `remote_runtime.rs`：内嵌 `pad.remote.v1` WSS LAN gateway、本机 leaf-DER 证书 pin、120 秒一次性扫码配对、设备 token hash、Profile 隔离、连续 revision/noop、慢客户端有界断开与本地远程管理动作；`remote/network.rs` 管理有界连接与 wire lifecycle，`remote/tls.rs` 管理可恢复的私有证书对，`remote/commands.rs`/`remote/receipts.rs` 管理 owner dispatch 与 receipt。Mutation 在执行前持久化 in-progress marker，重启后返回 `command_outcome_unknown` 而不自动重放，因此保证的是 at-most-once + resync，不声称跨 crash exactly-once。所有手机 command 仍经有界 channel 回到同一个 DesktopRuntime owner，remote 永远不能调用 destructive `poll`。
- `auth.rs`：Rust 唯一拥有的 Pi 登录/登出协调器；按 Profile 私有目录调用可信 Pi SDK，使用 attempt/prompt JSONL 状态机向 Desktop 暴露安全认证快照，不向 Electron 暴露认证路径、环境或凭据。
- `terminal.rs`：Rust 唯一拥有的单 pane PTY 桥；工作目录只从 Task Store 解析，renderer 不能传程序/cwd/env，输入、尺寸、pane 数与纯文本 snapshot 行数均有界。
- `data_root_lock.rs`：`${PAD_DESKTOP_DATA_DIR}/v1/store/desktop-server.lock` 非阻塞独占锁；一个数据根同一时间只允许一个 desktop-server 拥有 SQLite、Pi/auth 与 PTY 生命周期。
- `helpers.rs`：会话 JSONL 只读读取、Profile provider 认证摘要、路径边界和运行时状态转换等纯辅助逻辑。
- `tests.rs`、`bridge/tests.rs`、`bridge/tests/security.rs`、`bridge/tests/control_plane.rs`、`remote/tests/*.rs`：DesktopRuntime/JSONL/WSS bridge 的 focused contract cases，包含跨 Profile 列举、历史、运行时、认证、PTY 越权、配对、receipt、连接边界与 pending interaction 回归；所有 case 由 compact suite 显式注册执行。
- Desktop UI 后续只需通过该 facade 调用 start/prompt/poll/stop，不直接操作 Pi 子进程或 SQLite。

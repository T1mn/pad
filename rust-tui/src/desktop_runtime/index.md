# desktop_runtime

- `../desktop_runtime.rs`：PAD Desktop Control Plane 的最小 facade；组合私有 Store、Codex Sidebar snapshot、Pi Profile supervisor 与事件状态回写。
- `bridge.rs`：`pad __internal desktop-server` 的 stdin/stdout JSONL bridge；Swift/WKWebView 通过 bootstrap、Profile/Task CRUD（含 `set_profile` 策略持久化）、provider 状态、历史恢复、Pi 原生 RPC 和 guarded/full-access UI 响应访问 facade。
- `helpers.rs`：会话 JSONL 只读读取、Profile provider 认证摘要、路径边界和运行时状态转换等纯辅助逻辑。
- `tests.rs`、`bridge/tests.rs`：DesktopRuntime/JSONL bridge 的 focused contract cases。
- Desktop UI 后续只需通过该 facade 调用 start/prompt/poll/stop，不直接操作 Pi 子进程或 SQLite。

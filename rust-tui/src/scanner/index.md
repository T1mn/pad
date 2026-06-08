# scanner

- `../scanner.rs`：扫描 tmux panes 主流程，输出面板列表。
- `detect.rs`：根据 tmux current command、主进程、完整 args 与子进程识别 agent。
- `hydrate.rs`：批量补全 pane Busy/Waiting/Idle 状态与 Git 信息。
- `panel.rs`：把解析后的 tmux pane 行转换为基础 `AgentPanel`。
- `process_snapshot.rs` / `process_snapshot/`：用一次轻量进程快照读取本轮 pane 主/子进程，必要时才按单个 pid 补完整 args。
- `scan_caches.rs`：缓存进程快照与 Git 信息，避免同一轮扫描重复起子进程。
- `tmux_panes.rs` / `tmux_panes_tests.rs`：集中定义 tmux pane 输出格式和零分配行解析。
- `git.rs` / `git_tests.rs`：用 porcelain status 解析分支、commit 和变更数，并按唯一目录并发查询。
- `pane_capture.rs` / `pane_capture/`：批量截取 agent pane 尾部内容并清理 ANSI，用于 Busy/Waiting/Idle 判断。
- `tests.rs`：扫描、Git 解析与默认忽略的本机耗时测试，用于验证 TUI 卡顿优化。

# pi_runtime

- `mod.rs`：Pi 识别、Profile 目录解析和 `--mode rpc` 启动命令构建；不加载 Pi TUI。
- `jsonl.rs`：Pi RPC JSONL 编解码与帧大小边界。
- `events.rs` / `approval.rs`：运行事件归约、Full Access/Unattended 审批适配。
- `supervisor.rs`：线程安全的 Pi RPC 子进程监督器，隔离 stdout JSONL、stderr、退出和 generation。
- `session_index.rs`：只读扫描 Pi append-only session journal，PAD 仅保存可重建索引。

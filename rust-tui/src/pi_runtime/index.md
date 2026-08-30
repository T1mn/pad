# pi_runtime

- `mod.rs`：Pi 识别、Profile 目录解析、Desktop bundle 内可信 Pi launcher 选择和 `--mode rpc` 启动命令构建；不加载 Pi TUI。
- `jsonl.rs`：Pi RPC JSONL 编解码与帧大小边界。
- `events.rs` / `approval.rs`：运行事件归约、Pi UI/工具请求到结构化 `PolicyOperation` 的适配；适配器不自行决定 Full Access 放行。
- `supervisor.rs` / `supervisor/validation.rs` / `supervisor/tests.rs`：线程安全的 Pi RPC 子进程监督器，隔离 stdout JSONL、stderr、退出和 generation；validation 子模块负责无 shell 启动词法解析、RPC mode 与 Profile session/root 边界；Desktop 固定 launcher API 不接受 renderer command，并可把 Store 中已验证的 journal 作为原生 `--session` 参数传给 Pi，子进程使用 `077` umask；测试覆盖 Profile 根目录与 session 路径边界。
- `session_index.rs`：只读扫描 Pi append-only session journal，PAD 仅保存可重建索引。

# app/terminal

- `model.rs`：可序列化的 Shell/Codex/Claude/OpenCode/GitHub pane profile、稳定 ID、command 与二叉 split tree 基础模型。
- `model/workspace.rs`：tab/workspace 的 pane 分配、split、focus、关闭、恢复规范化与 invariant 校验。
- `agent_registry.rs`：把 launcher 和内置 Codex/Claude/OpenCode profile pane 登记到左侧 live sessions，并同步 label、焦点、跨 tab 跳转与关闭。
- `controller_io.rs`：单一 `TerminalController` / frame reader 上的 per-pane runtime 状态、frame polling 与公平背压 flush。
- `controller_io/queue.rs`：per-pane input/scroll/resize/label/close 的有界排队与合并。
- `interaction.rs`：终端命令层、rename 编辑和 sidebar/terminal 焦点切换。
- `runtime_io.rs`：终端输入、mouse、scroll、resize、frame polling、shutdown 与 workspace 持久化边界。
- `../terminal.rs`：workspace facade 与 Direct/Command/Rename 交互状态，按 clone→save→commit 提交布局变更，负责 pane 生命周期、OpenCode 配置命令的 runtime-only 解析、native agent live registry，以及 sidebar entry 到跨 tab pane 的聚焦/关闭同步。
- `tests.rs`：split 折叠与最小尺寸、tab/pane focus、ID 边界、可信 profile 恢复、OpenCode 完整命令、持久化失败回滚、重启生命周期、rename 目标和 per-pane 有界队列回归。

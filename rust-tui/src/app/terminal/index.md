# app/terminal

- `model.rs`：可序列化的 pane profile、稳定 ID、tab 与二叉 split tree，以及布局/深度/数量/序号 invariant 校验；恢复时从 profile 重新派生可信 command。
- `agent_registry.rs`：把 native agent pane 登记到左侧 live sessions，并同步 label、焦点、跨 tab 跳转与关闭。
- `controller_io.rs`：单一 `TerminalController` / frame reader 上的 per-pane runtime 状态、有界顺序输入与 scroll、resize/label 合并、关闭及公平背压重试。
- `../terminal.rs`：workspace facade 与 Direct/Command/Rename 交互状态，负责 pane 生命周期、持久化触发、native agent live registry，以及 sidebar entry 到跨 tab pane 的聚焦/关闭同步。
- `tests.rs`：split 折叠与最小尺寸、tab/pane focus、ID 边界、可信 profile 恢复、重启生命周期、rename 目标和 per-pane 有界队列回归。

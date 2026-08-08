# model

- `../model.rs`：模型模块入口，保留 `crate::model::*` 对外路径。
- `agent.rs` / `tests.rs` 内联 `agent`：agent 类型、状态、状态来源与 git 摘要模型。
- `preview.rs` / `tests.rs` 内联 `preview`：preview 来源、视图、turn 与共享 turn 列表。
- `panel.rs` / `tests.rs` 内联 `panel`：tmux/native agent live pane 展示模型与路径/git/uptime 文案。

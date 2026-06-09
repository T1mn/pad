# codex_turn_diff

- `mod.rs`：模块出口与内部 CLI。
- `model.rs`：Codex 单轮 diff 的 pending / completed / sider list 模型。
- `git.rs` / `git_tests.rs`：用临时 Git index 生成工作区 tree，并在两个 tree 之间生成 patch。
- `storage.rs` / `storage/` / `storage_paths.rs` / `storage_paths_tests.rs`：`~/.pad/codex-turn-diffs/` 下的 pending、patch、index 持久化与路径/key 生成。
- `recorder.rs`：处理 Codex hook 边界，`UserPromptSubmit` 记 baseline，`Stop` 落地单轮 patch。
- `tests.rs`：覆盖脏工作区隔离、未跟踪文件、pending live diff 与 CLI 入口。

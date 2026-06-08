# build

- `live.rs` / `live/`：运行中面板构建、归档隐藏判断与 live thread 组装。
- `seed.rs`：live/history/activity folder seed 入口。
- `sources.rs`：按视图预加载历史线程和 Codex session cache。
- `folder.rs`：单个 folder 内 live/history thread 合并流程。
- `finalize.rs` / `logging.rs`：folder 排序收尾与构建耗时日志。
- `history_claude.rs` / `history_codex.rs` / `history_codex/` / `history_gemini.rs` / `history_opencode.rs`：历史会话构建；Codex 拆分 merge、entry 与 session cache snapshot。
- `activity.rs` / `activity/` / `meta.rs`：活跃度合并、排序覆盖与元信息补全。
- `trash.rs` / `trash/`：回收区 folder 聚合与各 agent deleted thread 转换。
- `tests.rs` / `tests/`：构建流程、history entry、activity merge 与 meta 覆盖测试。

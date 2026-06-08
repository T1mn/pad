# session_continuity

- `model.rs` / `model/`：continuity ledger、snapshot、fallback 决策等数据结构。
- `health.rs`：lag/frozen 健康状态分类与 bootstrap 清理。
- `utils.rs`：时间、文本清理、transcript 元数据观察等小工具。
- `recording.rs` / `recording/`：hook/cache/preview assessment 记录入口与实现。
- `diagnostics.rs`：continuity 诊断事件构造与追加。
- `storage.rs` / `storage/`：continuity ledger 读写、snapshot 查找与诊断日志追加。
- `tests.rs`：健康状态分类、bootstrap 清理与 fallback 决策测试。

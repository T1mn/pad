# telegram/pending/approval

- `candidates.rs`：筛选需要扫描 Codex approval 的 pending request。
- `transcript.rs`：补全 approval 扫描需要的 transcript path 与初始 offset。
- `scan.rs`：扫描 transcript approval 更新，并回写 pending approval 状态。
- `notify.rs`：approval 状态变化后的 feedback 刷新、按钮消息、提示音与日志。

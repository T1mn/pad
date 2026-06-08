# session_loader

- `load.rs`：Session preview 主入口，串联缓存快路径、target resolve 与缺失 target fallback。
- `resolved.rs`：已解析 target 后的 transcript parse、continuity fallback 与空/错误分支。
- `cache.rs`：已确认 session preview 的缓存回退与 metadata 合并。
- `parse.rs`：按 AgentType 分发 transcript parser。
- `continuity.rs`：session continuity fallback 评估与记录。
- `persist.rs`：pane-origin resolved session 回写 session cache。
- `errors.rs`：session preview 不可用错误文案。
- `logging.rs`：session preview 慢路径、parse 结果与 continuity fallback 调试日志。

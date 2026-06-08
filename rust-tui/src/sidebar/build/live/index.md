# sidebar/build/live

- `fallback.rs`：没有历史分组时，按 live panel 工作目录构建 folder。
- `archived.rs`：判断 live panel 对应历史线程是否已归档，应从当前视图隐藏。
- `thread.rs`：把 `AgentPanel` 转换成 `SidebarThread`。
- `resolve.rs`：解析 live thread 的上游标题、subtitle 与 provider 名称。

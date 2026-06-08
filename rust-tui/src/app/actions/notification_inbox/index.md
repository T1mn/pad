# app/actions/notification_inbox

- `open.rs`：打开/关闭 notification inbox，并在重新加载后夹紧选中项。
- `selection.rs`：inbox 选中项移动与当前通知 id 读取。
- `mutate.rs`：标记已读、全部已读、删除和追加通知的 App 状态更新。
- `persist.rs`：把 inbox 变更写回持久化存储，并隔离测试环境开关。

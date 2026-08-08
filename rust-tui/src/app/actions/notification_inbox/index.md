# app/actions/notification_inbox

- `../notification_inbox.rs` 内联 `open`：打开/关闭 notification inbox，并在重新加载后夹紧选中项。
- `../notification_inbox.rs` 内联 `selection`：inbox 选中项移动与当前通知 id 读取。
- `../notification_inbox.rs` 内联 `mutate`：标记已读、全部已读、删除和追加通知的 App 状态更新。
- `../notification_inbox.rs` 内联 `persist`：把 inbox 变更写回持久化存储，并隔离测试环境开关。

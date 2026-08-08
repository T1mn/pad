# app/hooks/notification

- `../notification.rs` 内联 `request`：构造桌面通知标题与正文。
- `draft.rs`：把 pane/app-thread stop hook 转成收件箱草稿和待发送通知。
- `../notification.rs` 内联 `emit`：发送桌面通知并播放完成提示音。

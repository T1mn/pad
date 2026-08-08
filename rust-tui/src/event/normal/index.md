# event/normal

- `global_keys.rs` / `global_keys/`: normal mode global shortcuts, OpenCode actions, and numeric jumps.
- `preview_keys.rs`: focused preview scrolling, turn selection, and expansion keys.
- `sidebar_keys.rs`: sidebar/tree navigation, folder toggles, delete, native pane focus, and stale external live-entry notice.
- `../normal.rs` 内联 `tab`: Tab focus switching and double-Tab preview detail behavior.
- `terminal_keys.rs` / `terminal_keys_tests.rs`：focused Native terminal 输入与 Shift scrollback；F12 返回左侧，F11（或增强键盘协议下的 `Ctrl+Shift+Space`）打开 tab/split/profile/focus/rename/close 命令层，并覆盖关键路由回归。

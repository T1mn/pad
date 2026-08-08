# notify

- `../notify.rs` 内联 `command`：通知子进程启动、PATH 可执行文件探测。
- `../notify.rs` 内联 `linux`：Linux 桌面环境判断与 `notify-send` 命令参数。
- `../notify.rs` 内联 `macos`：macOS `osascript` 通知命令参数。
- `tests.rs`：平台命令选择与 PATH 探测回归。

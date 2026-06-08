# system_check

- `compatibility.rs`：tmux 能力探测、必需/可选能力缺失错误格式化。
- `install.rs` / `install/`：tmux 是否存在、安装方式探测和安装步骤执行。
- `tests.rs`：安装方式选择和确认输入测试。
- 上层 `system_check.rs`：启动前 tmux 可用性入口和 `tmux doctor` 入口。

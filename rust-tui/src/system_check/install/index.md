# system_check/install

- `model.rs`：tmux 安装方式枚举与手动安装提示。
- `detect.rs`：检测 tmux 是否存在，并按 OS/包管理器选择安装方式。
- `steps.rs`：把安装方式展开成命令步骤并执行。

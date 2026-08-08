# system_check/install

- `../install.rs` 内联 `model`：tmux 安装方式枚举与手动安装提示。
- `../install.rs` 内联 `detect`：检测 tmux 是否存在，并按 OS/包管理器选择安装方式。
- `../install.rs` 内联 `steps`：把安装方式展开成命令步骤并执行。

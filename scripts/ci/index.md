# ci

- `native_terminal_smoke.py`：通过伪终端验证 PAD 原生终端的启动、Shell 输入、焦点返回与退出。
- `linux_installer_smoke.sh`：Linux 安装器冒烟测试。
- `mock_agent.sh`：CI 用 mock agent。
- `power_smoke.py`：pad 原生终端空闲 CPU 预算测试，作为耗电量代理指标。
- `check_index.py`：检查工作区每个目录都有短小的 `index.md`，并捕获未暂存文件的过期引用。
- `check_rust_file_size.py`：生产 Rust 文件上限 500 行、测试文件（含 tests 与 *_tests 目录）上限 800 行、全局绝对上限 1000 行，并防止生产源码重新内嵌 `mod tests {}`；翻译表仅豁免分类上限。

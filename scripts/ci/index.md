# ci

- `native_terminal_smoke.py`：通过伪终端验证 PAD 原生终端的启动、Shell 输入、焦点返回与退出。
- `linux_installer_smoke.sh`：Linux 安装器冒烟测试。
- `mock_agent.sh`：CI 用 mock agent。
- `power_smoke.py`：pad 原生终端空闲 CPU 预算测试，作为耗电量代理指标。
- `check_index.py`：检查仓库、crate 与一级功能模块都有短小的 `index.md`，并捕获未暂存文件的过期引用；实现叶子不再强制重复建索引。
- `check_rust_file_size.py`：合并后的 Rust 功能模块上限 800 行、全局绝对上限 1000 行；允许测试紧邻实现以减少跨文件检索，翻译表仅豁免分类上限。
- `check_test_suites.py`：锁定 100 个 Rust 测试入口；93 个同步功能域套件继续调用 763 个原 case，另保留 5 个异步测试和 2 个 ignored 基准。

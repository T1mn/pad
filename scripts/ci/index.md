# ci

- `native_terminal_smoke.py`：通过伪终端验证 PAD 原生终端的启动、Shell 输入、焦点返回与退出。
- `pad_desktop_e2e_smoke.py`：macOS 打包 App 黑盒测试，覆盖 arm64/签名、Desktop server、Profile 隔离、Full Access 策略、guarded approval round-trip、Task stop/reopen、Pin/Archive/Unread 持久化、Pi session/history 恢复和 confirm/select/input UI 请求行为。
- `pad_desktop_electron_e2e.py`：最终 Electron `.app` 黑盒验收；验证 arm64/签名/Fuses/随包运行时、Protocol v2 Rust sidecar、Profile A/B 与独立 SQLite；12 类 synthetic Codex/ChatGPT/custom `CODEX_HOME` 均须拒绝作为数据根且内容/元数据不变；并通过 CDP 留存宽窄窗口 DOM、截图、中文可访问性树和零残留进程证据。
- `pad_desktop_visual.py`：最终 Electron `.app` 视觉矩阵门禁；在隔离 HOME 中采集 light/dark × 五档窗口截图，检查平铺/overlay 布局、DOM 几何、中文动作、ARIA/focus-visible 与水平溢出，并只对用户提供的同名基线计算真实 SSIM/像素差。
- `pad_desktop_perf.py`：最终 Electron `.app` 性能门禁；使用临时 HOME/CDP 测量 renderer 可交互冷启动与独立 bootstrap、10 秒空闲进程树 CPU/RSS 和 Renderer JS heap，并把 3 秒/2%/450 MiB、Protocol v2、backend ready 与零错误提示判定写入 JSON。
- `linux_installer_smoke.sh`：Linux 安装器冒烟测试。
- `mock_agent.sh`：CI 用 mock agent。
- `power_smoke.py`：pad 原生终端空闲 CPU 预算测试，作为耗电量代理指标。
- `check_index.py`：检查仓库、crate 与一级功能模块都有短小的 `index.md`，并捕获未暂存文件的过期引用；实现叶子不再强制重复建索引。
- `check_rust_file_size.py`：合并后的 Rust 功能模块上限 800 行、全局绝对上限 1000 行；允许测试紧邻实现以减少跨文件检索，翻译表仅豁免分类上限。
- `check_test_suites.py`：锁定 100 个 Rust 测试入口；93 个同步功能域套件继续调用 772 个 case，另保留 5 个异步测试和 2 个 ignored 基准。

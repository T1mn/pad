# workflows

- `ci.yml`：主 CI，文档类变更跳过重型矩阵，Linux prebuilt fixtures 并行构建，含 cargo audit 依赖漏洞门禁。
- `release.yml`：发布流程，直接用 dist 产物跑原生终端 smoke，并把 pad power 指标写入 release body。
- `native-terminal-smoke.yml`：在 macOS 与 Linux 上验证 PAD 原生 PTY 交互，文档类变更跳过。

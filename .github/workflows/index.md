# workflows

- `ci.yml`：主 CI，文档类变更跳过重型矩阵，Linux prebuilt fixtures 并行构建。
- `release.yml`：发布流程，直接用 dist 产物跑 smoke，并把 pad / pad-sider power 指标写入 release body。
- `tmux-smoke.yml`：tmux 冒烟验证，文档类变更跳过。

# workflows

- `ci.yml`：主 CI，文档类变更跳过重型矩阵，并缓存 cross。
- `release.yml`：发布流程，缓存 cross 编译工具。
- `tmux-smoke.yml`：tmux 冒烟验证，文档类变更跳过。

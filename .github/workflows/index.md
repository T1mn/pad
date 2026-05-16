# workflows

- `ci.yml`：主 CI，文档类变更跳过重型矩阵，并缓存 cross。
- `release.yml`：发布流程，缓存 cross 编译工具，并把 pad / pad-sider power smoke 指标写入 release body。
- `tmux-smoke.yml`：tmux 冒烟验证，文档类变更跳过。

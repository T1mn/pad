# AGENTS

## 项目习惯

- 看代码前先读相关目录的 `index.md`，结构变化后同步更新。
- 普通代码文件避免无意义膨胀；文档保持短小、可定位。
- CI / release 问题先确认远程失败点，再本地复现和验证。

## 版本与 hotfix 原则

- 默认优先升正式 patch 版本，例如 `0.6.16` -> `0.6.17`，标签用 `v0.6.17`。
- `hotfix` 只用于同一正式版本刚发布后的紧急小修，且应是单点修复。
- 同一正式版本连续 hotfix 超过 2 个，或同时包含 CI、release、性能、用户可见行为等多类变化时，停止继续叠 `hotfixN`，改升下一个 patch 版本。
- 已推送的失败 tag 不要反复修补旧 tag；应修到 `master`，升版本，再打新 tag 触发 release。
- 发布前按 `docs/release-checklist.md` 验证，确认 `rust-tui/Cargo.toml` 版本与 tag 匹配。

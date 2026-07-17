# Agent compatibility audit

审计日期：2026-07-17。这里的“支持”指 PAD 已接入对应能力，不代表 agent
官方承诺兼容 PAD。

## 版本基线

| Agent / 官方分发 | 稳定版（发布时间） | 本机版本 | PAD 状态 |
| --- | --- | --- | --- |
| [OpenAI Codex CLI](https://github.com/openai/codex/releases/tag/rust-v0.144.5) / `@openai/codex` | `0.144.5`（2026-07-16） | `0.144.3` | 部分 |
| [Anthropic Claude Code](https://github.com/anthropics/claude-code/releases/tag/v2.1.205) / `@anthropic-ai/claude-code` | stable `2.1.205`（2026-07-08）；rolling `2.1.212`（2026-07-16） | `2.1.204` | 部分 |
| [xAI Grok Build](https://x.ai/cli) / 官方平台二进制 | [stable `0.2.102`](https://x.ai/cli/stable)（2026-07-17） | `0.2.101` | 部分 |
| [OpenCode](https://github.com/anomalyco/opencode/releases/tag/v1.18.3) / `opencode-ai` | `1.18.3`（2026-07-16） | `1.17.3` | 部分 |

本机 CLI 没有被自动升级；最新版本通过官方包、release、帮助输出和格式样本审计。
Claude 的 rolling 版不是本轮稳定兼容基线。

## 能力矩阵

“完整”表示该能力已接入；“部分”表示只覆盖已知稳定格式；“不支持”表示 PAD
没有入口；“未验证”表示本轮没有完成最新版实机验证。

| 能力 | Codex | Claude Code | Grok Build | OpenCode |
| --- | --- | --- | --- | --- |
| 进程 / tmux pane 识别 | 部分 | 部分 | 完整 | 完整 |
| agent 类型 / 状态识别 | 部分 | 部分 | 部分 | 部分 |
| 启动命令及参数 | 完整 | 完整 | 完整 | 完整 |
| attach / detach | 完整 | 完整 | 完整 | 完整 |
| session ID 提取 | 部分 | 部分 | 部分 | 部分 |
| 恢复会话 | 完整 | 完整 | 完整 | 完整 |
| history / preview | 完整 | 完整 | 部分 | 完整 |
| hook / event | 部分 | 部分 | 不支持 | 不支持 |
| 配置 / relay | 部分 | 部分 | 不支持 | 部分 |
| archive / restore | 部分 | 完整 | 不支持 | 部分 |
| export / import | 不支持 | 不支持 | 不支持 | 完整 |
| 版本检测 / 更新提示 | 完整 | 不支持 | 不支持 | 不支持 |
| 损坏数据 / 未知字段降级 | 部分 | 部分 | 完整 | 部分 |
| 最新稳定二进制 smoke | 完整 | 部分 | 完整 | 完整 |

## 边界

- Codex `0.144.5` 的冷历史可能是 `.jsonl.zst`；PAD 可读预览，但不会重写压缩
  rollout 来做 provider 同步。archive 仍直接操作上游文件和 SQLite，因此标为部分。
- Claude stable 支持 `CLAUDE_CONFIG_DIR`；rolling `2.1.212` 只检查了接口变化，未作为
  release 基线。PAD 不自动注册 Claude hook，relay 关闭时恢复整份备份仍可能覆盖并发修改。
- Grok 只接入官方 session summary / `updates.jsonl` 的已知消息格式；未知事件会跳过，
  live session ID 和 working/waiting 不能从磁盘可靠得出，且不自动写用户 hooks。
  详见 [Grok 支持说明](grok-support.md)。
- OpenCode history 依赖官方本地 SQLite 数据；数据库 schema 变化仍需重新审计。
  详见 [OpenCode 支持说明](opencode-support.md)。
- live pane 仅在 cwd 唯一时回退；同 cwd 多 session 没有明确 ID 时退回 tmux 预览。

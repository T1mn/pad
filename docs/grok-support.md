# Grok Build support

PAD 以 xAI 官方 [Grok Build 文档](https://docs.x.ai/build/overview)、
[CLI 页面](https://x.ai/cli)和[源码](https://github.com/xai-org/grok-build)为准。
2026-07-17 审计 stable `0.2.102`；本机是 `0.2.101`，未自动升级。

## 已接入

- 识别 `grok` 进程，并创建或 attach tmux pane。
- 用 `grok --resume <session-id>` 恢复会话；`--session-id` 只用于新会话，不用于恢复。
- 从 `$GROK_HOME/sessions/`（默认 `~/.grok/sessions/`）读取 history。
- 从 session 的 `summary.json` 和 `updates.jsonl` 生成侧栏信息与最近问答预览。
- 单个损坏 session 或未知 update 会被跳过，不阻塞其他历史。

## 暂不支持

- 不自动创建或修改 `$GROK_HOME/hooks/*.json`。
- 不接入 Grok relay / model 配置。
- 不提供 Grok archive / restore、export / import。
- 不承诺解析未公开或未来新增的 update 类型。

Grok 没有可供 PAD 外部接管的专用 attach API；PAD 的 attach 指进入原 tmux pane。

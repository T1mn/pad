# OpenCode support

PAD 对 OpenCode 的支持以官方 CLI 能力和本机 `opencode 1.15.13` 验证为基准。
官方 CLI 文档入口：<https://opencode.ai/docs/cli/>。

## 已接入能力

- 启动 / attach：把 `opencode` 作为普通 agent pane 管理。
- relay/model：在 PAD relay 设置中维护 OpenCode provider/model，并写入 `opencode.jsonc`。
- history：读取 OpenCode 官方 SQLite 数据库，合入 session 侧栏。
- preview：从 `session` / `message` / `part` 表解析最近问答。
- resume：通过 `opencode --session <session_id>` 恢复会话。
- archive：使用 OpenCode session 的 `time_archived` 字段归档 / 恢复。
- metadata：展示并搜索 share URL、cost、input/output/reasoning/cache tokens。
- share：预览卡点击 `SHARE` 可复制完整 URL。
- export：`E` 调用 `opencode export <session>`，保存到 `~/.pad/opencode-exports/`。
- sanitized export：`S` 调用 `opencode export <session> --sanitize`。
- import：`I` 从剪贴板读取 JSON 路径或 OpenCode share URL，调用 `opencode import <file>`。
- stats：`O` 调用 `opencode stats --project <cwd> --models 10 --tools 10`，保存到 `~/.pad/opencode-stats/`。
- diagnostics：`P` 调用 `opencode providers list` 和 `opencode models --verbose`，保存到 `~/.pad/opencode-diagnostics/`。

## 已验证但暂未接入

- `opencode session list --format json`：本机可用但当前没有稳定输出可替代 SQLite history；PAD 继续以 SQLite 为主。
- `opencode session delete <sessionID>`：破坏性强，PAD 现有 `d` 是隐藏/删除 pane，不直接删除 OpenCode 官方 session。
- `opencode serve` / `web` / `attach <url>`：适合远程/headless 工作流；需要 server 生命周期、认证和 URL 输入设计后再接。
- `opencode run`：适合非交互自动化；和 PAD 当前 pane/TUI 工作流不同，暂不混入普通快捷键。
- `opencode mcp` / `plugin` / `agent`：属于 OpenCode 配置管理，未来更适合放进设置页。`providers list` 和 `models --verbose` 已作为只读诊断导出。
- `opencode db`：调试用途强，PAD 已直接读取 SQLite，暂不暴露任意 DB 查询入口。

## 本地验证命令

```bash
~/.opencode/bin/opencode --version
~/.opencode/bin/opencode --help
~/.opencode/bin/opencode export --help
~/.opencode/bin/opencode import --help
~/.opencode/bin/opencode stats --help
~/.opencode/bin/opencode providers list
~/.opencode/bin/opencode models --verbose
~/.opencode/bin/opencode session --help
~/.opencode/bin/opencode session list --format json --max-count 5
```

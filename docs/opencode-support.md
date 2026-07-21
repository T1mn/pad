# OpenCode support

PAD 对 OpenCode 的支持以 2026-07-17 官方 stable `1.18.3` 审计为基准；本机
`opencode 1.17.3` 没有被自动升级。
官方 CLI 文档入口：<https://opencode.ai/docs/cli/>。

## 已接入能力

- 启动 / attach：把 `opencode` 作为普通 agent pane 管理。
- relay/model：在 PAD relay 设置中维护 OpenCode provider/model，并写入 `opencode.jsonc`。
- history：读取 OpenCode 官方 SQLite 数据库，合入 session 侧栏。
- preview：从 `session` / `message` / `part` 表解析最近问答。
- resume：通过 `opencode --session <session_id>` 恢复会话。
- archive：使用 OpenCode session 的 `time_archived` 字段归档 / 恢复，不改 `time_updated` 排序。
- metadata：展示并搜索 share URL、cost、input/output/reasoning/cache tokens。
- share：预览卡点击 `SHARE` 可复制完整 URL。
- export：`E` 调用 `opencode export <session>`，保存到 `~/.pad/opencode-exports/`。
- sanitized export：`S` 调用 `opencode export <session> --sanitize`。
- import：`I` 从剪贴板读取 JSON 路径或 OpenCode share URL，调用 `opencode import <file>`。
- GitHub agent install：`H` 在当前选中线程工作目录新开 tmux window 调用 `opencode github install`，交给 OpenCode 完成 GitHub agent 安装流程。
- plugin：`L` 从剪贴板读取 npm module 名称，在当前选中线程工作目录新开 tmux window 调用 `opencode plugin <module>`；不默认追加 `--global` 或 `--force`。
- PR：`G` 从剪贴板读取 PR 编号、`#123` 或 GitHub `/pull/<number>` URL，在当前选中线程工作目录新开 tmux window 调用 `opencode pr <number>`。该命令会由 OpenCode fetch/checkout PR 分支。
- run：`X` 从剪贴板读取 prompt，在当前选中线程工作目录新开 tmux window 调用 `opencode run <prompt>`；如果选中的是 OpenCode 会话，会追加 `--session <session_id>`。
- serve：`B` 在当前选中线程工作目录新开 tmux window 调用 `opencode serve --hostname 127.0.0.1 --port 0`，启动仅本机可访问的随机端口 headless server；关闭窗口即停止。
- stats：`O` 在选中线程工作目录调用 `opencode stats --project "" --models 10 --tools 10`，按 OpenCode 当前项目统计并保存到 `~/.pad/opencode-stats/`。
- diagnostics：`P` 调用 `opencode --version`、`db path`、`debug info`、`debug paths`、`debug config`、`providers list`、`models --verbose`、`agent list`、`mcp list`，保存到 `~/.pad/opencode-diagnostics/`。
- remote attach：`Y` 从剪贴板读取 http(s) server URL，新开 tmux window 调用 `opencode attach <url>`；工作目录优先使用当前选中线程。
- web：`W` 在当前选中线程工作目录新开 tmux window 调用 `opencode web`，由 OpenCode 启动本地 server 并打开浏览器。

## 已验证但暂未接入

- `opencode session list --format json`：本机可用但当前没有稳定输出可替代 SQLite history；PAD 继续以 SQLite 为主。
- `opencode session delete <sessionID>`：破坏性强，PAD 现有 `d` 是隐藏/删除 pane，不直接删除 OpenCode 官方 session。
- `opencode serve` 的远程暴露、固定端口、认证和复用策略：PAD 目前只提供本地随机端口窗口入口；跨机器/headless 常驻需要显式设计后再接。
- `opencode github run`：涉及 token 或 mock event 输入，未来更适合进入专门的 GitHub 工作流设置；`opencode github install` 和 `opencode pr <number>` 已作为显式入口接入。
- `opencode plugin` 的 `--global` / `--force`：PAD 目前只接默认项目级安装，避免隐式全局写入或覆盖。
- MCP / agent 的增删改认证命令：属于 OpenCode 配置管理，未来更适合放进设置页。`agent list`、`mcp list`、`providers list` 和 `models --verbose` 已作为只读诊断导出。
- `opencode db`：调试用途强，PAD 已直接读取 SQLite，暂不暴露任意 DB 查询入口。

## 兼容与安全边界

- history / preview 直接读取 OpenCode 本地 SQLite；未来 schema 变化需要重新审计。
- PAD 不接入 OpenCode hook 实时事件；live 状态仍以 tmux pane 为准。
- relay 写配置前必须能解析现有 `opencode.jsonc`；解析失败时应停止，不覆盖用户文件。
- 启动 pad 时只应用 runtime permission overlay，不写 OpenCode live provider/model；provider 同步仅在用户改 relay、外部 reload 配置、或 launcher 选中 OpenCode 时发生。
- diagnostics 即使经过敏感字段清理，分享前仍应人工检查；PAD 不自动上传诊断文件。
- remote attach 只接受用户显式提供的 URL；PAD 不自动开放 server 到公网。

## 本地验证命令

```bash
~/.opencode/bin/opencode --version
~/.opencode/bin/opencode --help
~/.opencode/bin/opencode export --help
~/.opencode/bin/opencode import --help
~/.opencode/bin/opencode plugin --help
~/.opencode/bin/opencode pr --help
~/.opencode/bin/opencode run --help
~/.opencode/bin/opencode attach --help
~/.opencode/bin/opencode web --help
~/.opencode/bin/opencode serve --help
~/.opencode/bin/opencode stats --help
~/.opencode/bin/opencode db path
~/.opencode/bin/opencode debug info
~/.opencode/bin/opencode debug paths
~/.opencode/bin/opencode debug config
~/.opencode/bin/opencode providers list
~/.opencode/bin/opencode models --verbose
~/.opencode/bin/opencode agent list
~/.opencode/bin/opencode mcp list
~/.opencode/bin/opencode mcp add --help
~/.opencode/bin/opencode agent create --help
~/.opencode/bin/opencode session --help
~/.opencode/bin/opencode github install --help
~/.opencode/bin/opencode github run --help
~/.opencode/bin/opencode session list --format json --max-count 5
```

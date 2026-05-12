# CODEX_HOME 隔离计划（实现后删除）

## 目标

- pad 启动的 Codex 固定使用 `CODEX_HOME=~/.pad/codex-home`。
- 保证目录启动前存在，缺失时自动创建。
- 避免 Codex App / 正式账号污染 pad 的 CLI 鉴权配置。
- 尽量让 session 能继续统一读取，不因切换账号或 Codex Home 改变而丢失。

## 初步判断

- `CODEX_HOME` 是 Codex CLI 的用户数据目录，默认通常是 `~/.codex`。
- 隔离后，`config.toml`、`auth.json`、hooks、prompt 等会优先从 `~/.pad/codex-home` 读取。
- 如果 session 文件也跟随 `CODEX_HOME` 存放，直接切换会导致旧 session 看不见。
- 所以不能只改环境变量，还要处理 session 目录的兼容。

## 实现方案

1. 新增 pad 专用 Codex Home：`~/.pad/codex-home`。
2. pad 启动 / 重启 Codex 前统一注入环境变量：
   - `CODEX_HOME=~/.pad/codex-home`
3. 初始化该目录：
   - 复制或生成 pad 需要的 `config.toml`。
   - 写入 pad 管理的 hooks / prompt 配置。
   - 不复制正式 Codex App 的 `auth.json` token。
4. session 保持统一：
   - 优先确认 Codex session 的真实存储路径。
   - 如果 session 默认在 `~/.codex` 下，则在 `~/.pad/codex-home` 中使用软链接指向原 session 目录。
   - 这样正式 Codex 和 pad Codex 都能读同一批 session。
5. 重启 Codex 时继续使用原 session id：
   - preview 不动。
   - tmux pane 内只重启 Codex 进程。
   - 仍走 `codex resume <session_id>`。

## 风险点

- 新旧 Codex 版本的 session 目录结构可能不同，需要先探测再决定软链接位置。
- `auth.json` 不应共享，否则仍可能出现 ChatGPT token / CPA key 冲突。
- hooks 配置要写入隔离后的 `CODEX_HOME`，否则新 Codex 读不到。

## 验证

- 新开 pad Codex 时环境中能看到 `CODEX_HOME=~/.pad/codex-home`。
- CPA 请求不再使用正式账号 token。
- 旧 session 可以 resume。
- Shift+R 重启后 preview 不受影响，Codex 仍继承原 session。

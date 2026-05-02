# src

- `main.rs`：入口、tmux bootstrap、隐藏内部子命令分发与主循环。
- `app/`：应用状态、导航、预览、hook 与异步任务。
- `ui/`：TUI 布局、状态栏、预览、设置弹窗。
- `theme/`：配置模型、加载保存、主题。
- `pad_sider/`：Codex pane 左侧辅助栏，负责 tree、Markdown 预览、`/` fuzzy 搜索与 changed files。
- `relay/`：各 agent relay/native 配置与运行时覆盖。
- `preview_source/`：Claude/Codex/Gemini 会话预览解析。
- `chat/`：聊天后端与 Telegram 集成。
- `sidebar/`：侧边栏历史、搜索、provider 展示。
- `session_cache/`：会话快照缓存与持久化。
- `paths.rs` / `paths/`：运行目录、prompt、hook bridge 路径与安装。
- `scanner.rs`：tmux pane 扫描与 agent 识别。
- `event.rs` / `event/`：键鼠事件分发。

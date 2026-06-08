# session

- `detail.rs` / `detail/`：Session detail 视口绘制与 Markdown turn 行构建。
- `list.rs` / `list/`：Session list 绘制入口，拆分卡片/gap 渲染与行数/命中布局。
- `scroll.rs`：Session list/detail 滚动裁剪与可见窗口计算。
- `badges.rs`：预览信息卡 badge、状态文案与 agent 配色。
- `text.rs`：问答前缀清理、卡片文本按宽切行。
- `tests.rs`：Session list/detail 渲染与命中测试。

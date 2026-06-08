# ui/panel_list/draw

- `block.rs`：左侧 panel list 的标题、边框与焦点样式。
- `content.rs`：空态或可见窗口内的 table 渲染，并返回 offset/scrollbar 状态。
- `row.rs`：根据 sidebar item 分发到 folder/thread row renderer。
- `scrollbar.rs`：列表超高时渲染右侧 scrollbar。

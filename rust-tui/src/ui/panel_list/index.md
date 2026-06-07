# ui/panel_list

- `mod.rs`: facade，导出面板列表、文件树与状态栏绘制入口。
- `draw.rs`: 左侧会话列表 block、可见窗口、scrollbar 与行分发。
- `empty.rs` / `labels.rs`: 空列表文案与特殊视图标题。
- `file_tree.rs` / `status.rs`: 文件树 fallback 与 agent 数量状态栏。
- `folder_row.rs`: folder/group row rendering。
- `thread_row.rs` / `thread_subtitle.rs`: thread row rendering、subtitle/tags 与 jump badges。
- `viewport.rs`: only build rows around the visible sidebar selection to keep redraws cheap。
- `width.rs`: cache preferred sidebar width so layout does not rescan visible rows every frame。
- `style.rs`: sidebar colors and shared style helpers。
- `animation.rs`: busy/waiting badge animation。
- `metrics.rs`: display width and truncation helpers。
- `tests.rs` / `thread_row_tests.rs`: panel list rendering helpers tests。

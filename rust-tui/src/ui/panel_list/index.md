# ui/panel_list

- `../panel_list.rs`: facade，导出面板列表、文件树与状态栏绘制入口。
- `draw.rs` / `draw/`: 左侧会话列表入口，拆分 block、content/window、row 分发与 scrollbar。
- `empty.rs` / `labels.rs`: 空列表文案与特殊视图标题。
- `file_tree.rs` / `status.rs`: 文件树 fallback 与 agent 数量状态栏。
- `folder_row.rs` / `folder_row_tests.rs`: folder/group row rendering。
- `thread_row.rs` / `thread_row/`：thread 单行 title rendering 与 jump badges。
- `viewport.rs` / `viewport_tests.rs`: only build rows around the visible sidebar selection to keep redraws cheap。
- `width.rs` / `width_tests.rs`: cache preferred sidebar width so layout does not rescan visible rows every frame，并覆盖缓存失效测试。
- `style.rs`: sidebar colors and shared style helpers。
- `animation.rs`: busy/waiting badge animation。
- `metrics.rs`: display width and truncation helpers。
- `tests.rs` / `thread_row_tests.rs`: panel list rendering helpers tests。

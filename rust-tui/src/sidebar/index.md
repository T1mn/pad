# sidebar

- `mod.rs`：侧边栏 facade，外露构建、模型、搜索、排序和标题清理入口。
- `model.rs` / `model/`：侧边栏展示模型、folder/thread/item helper。
- `display.rs` / `provider.rs` / `sort.rs`：显示、provider 汇总与排序。
- `search.rs` / `search/`：侧边栏搜索、可见项构建与 subagent source 判断。
- `gemini.rs`：Gemini 专用侧边栏处理。
- `build/history_opencode.rs`：OpenCode SQLite 历史合入侧边栏。
- `build/`：历史、live、trash 等构建流水线。

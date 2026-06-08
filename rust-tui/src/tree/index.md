# tree

- `../tree.rs`：FileTree / TreeEntry / TreeMode 状态定义与构造入口。
- `navigation.rs` / `navigation/`：目录扫描、进入/返回、展开与上下选择，按 scan/travel/selection 拆分。
- `search.rs`：tree 搜索模式、输入处理与过滤。
- `render.rs`：tree 列表渲染与文件类型图标。
- `preview_type.rs`：文件预览类型识别。
- `agent_launcher.rs`：从 tree 目录打开 agent 选择器的状态、默认 agent 列表与选择移动。

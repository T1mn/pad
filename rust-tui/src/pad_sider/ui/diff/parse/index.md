# pad_sider/ui/diff/parse

- `title.rs`：从 `diff --git` 行解析右侧展示文件名。
- `hunk.rs`：解析 hunk 起始行号并把 patch 行转换成结构化 diff row。
- `pairing.rs`：把相邻 delete/add 行配对成 change row，供 side-by-side 展示。

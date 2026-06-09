# tree

- `model.rs`：左侧文件树可见行模型。
- `build.rs` / `build_tests.rs`：按展开状态构建可见 tree rows，并在排序前跳过忽略目录。
- `scan.rs` / `scan_tests.rs`：递归扫描文件列表，供 `/` fuzzy 搜索使用。
- `ignore.rs`：tree 和文件扫描共享的忽略目录规则。

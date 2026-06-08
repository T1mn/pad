# tree

- `model.rs`：左侧文件树可见行模型。
- `build.rs`：按展开状态构建可见 tree rows，并缓存目录项类型用于排序。
- `scan.rs`：递归扫描文件列表，供 `/` fuzzy 搜索使用。
- `ignore.rs`：tree 和文件扫描共享的忽略目录规则。

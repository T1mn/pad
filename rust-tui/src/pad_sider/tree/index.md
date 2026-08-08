# tree

- `../tree.rs` 内联 `model`：左侧文件树可见行模型。
- `build.rs` / `tests.rs` 内联 `build`：按展开状态构建可见 tree rows，并在排序前跳过忽略目录。
- `scan.rs` / `tests.rs` 内联 `scan`：递归扫描文件列表，供 `/` fuzzy 搜索使用。

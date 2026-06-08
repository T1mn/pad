# scanner/process_snapshot/loader

- `command.rs`：执行轻量/完整 `ps` 快照命令，并把 stdout 交给解析器。
- `parse.rs`：解析 `pid ppid command` 行，生成 command map 与直接 child pid map。
- `filter.rs`：root pid 归一化，以及按 root/直接子进程过滤快照行。

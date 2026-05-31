# process_snapshot

- `loader.rs`：用一次轻量 `ps -axo pid,ppid,comm` 读取本轮 pane 主进程和直接子进程，必要时才回退完整 args。
- `classify.rs`：判断哪些命令可能需要补完整 args，避免每轮对普通 shell 做无效 `ps -p`。
- `../process_snapshot.rs`：懒加载进程快照、按需补全单个进程完整命令，用于 agent 识别。

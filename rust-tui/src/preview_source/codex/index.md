# codex

- `parser.rs` / `parser/`：Codex JSONL transcript 解析；普通文件走尾读，冷存储 `.jsonl.zst` 走流式解压，并只保留最近预览 turn。
- `tail.rs` / `tail_tests.rs`：按尾部窗口读取 Codex JSONL，拿够最近预览需要的用户消息后停止，避免每次预览重扫超大 transcript。
- `status_probe.rs`：通过 `/status` 探测 live Codex session id，并用短 TTL 缓存避免频繁打扰 pane。
- `normalize.rs` / `normalize/`：清理 Codex 用户消息里的环境块、shell 包装与图片引用，输出适合预览/标题的文本。
- `subagent.rs` / `subagent_tests.rs`：压缩 Codex subagent notification，合并到主 turn 预览。
- `tests.rs` / `tests/`：Codex transcript、normalize、status probe 和 ignored 解析基准测试。

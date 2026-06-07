# sound

- `../sound.rs`：声音功能入口，负责开关判断、运行时资源生成和平台播放调度。
- `catalog.rs`：音效事件、preset 元数据与 preset ID 归一化。
- `wav.rs`：按 preset 合成 WAV bytes。
- `playback.rs`：平台播放命令选择、进程启动与 PATH 可执行检测。
- `test_capture.rs`：测试期播放捕获工具。
- `tests.rs`：声音资源生成、开关与平台命令选择测试。

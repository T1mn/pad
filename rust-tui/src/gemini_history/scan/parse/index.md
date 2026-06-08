# gemini_history/scan/parse

- `snapshot.rs`：解析单个 Gemini session JSON 为 `GeminiSnapshot`。
- `project.rs`：从 session 文件路径推断 project root 与 alias。
- `message.rs`：归一化 Gemini message/summary 文本结构。

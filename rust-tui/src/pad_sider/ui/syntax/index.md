# syntax

- `mod.rs`：代码高亮入口。
- `language.rs`：按文件名/扩展名识别语言、标签与文件树强调色。
- `render.rs`：轻量 token 高亮，把代码行转成 `ratatui` spans。
- `lex.rs`：字符串、注释、数字和操作符等基础词法辅助。
- `tokens.rs`：各语言关键词、类型词与常用内建词。
- `styles.rs`：VS Code Dark+ 风格配色。
- `tests.rs`：语言识别和高亮颜色回归测试。

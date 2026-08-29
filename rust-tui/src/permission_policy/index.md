# permission_policy

- `../permission_policy.rs`：Profile→Project→Task 权限层级合并、默认保护命名空间与最终 Allow/Prompt/Deny 决策。
- `model.rs`：Pi 会话元数据、Profile/Project/Task 与自定义侧边栏 Section 的持久化 DTO。
- `path.rs`：词法路径归一化、existing-prefix canonicalize、symlink 安全边界与 workspace 风险分类。
- `shell.rs`：只接受单条纯字面量 argv 的 shell 静态检查；变量、替换、重定向、控制符和嵌套 evaluator 不会自动确认。

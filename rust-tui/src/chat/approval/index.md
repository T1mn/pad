# approval

- `answers.rs`：扫描 Codex transcript 中的 task_complete/final_answer 文本。
- `failures.rs`：扫描 Codex error event，并维护 failure scan offset。
- `requests.rs`：扫描 Codex require_escalated approval 请求和 resolved call_id。

# event/modes

- `thread_action_confirm.rs` / `thread_action_confirm_tests.rs`: thread actions plus title/tag editor keys and editor clear tests.
- `settings/`: settings modal key handling.
- `relay_settings/`: relay/provider editor key handling.
- `../modes.rs` 内联 `search` / `../modes.rs` 内联 `tree_search`: search modes.
- `tree.rs` / `../modes.rs` 内联 `file_preview`: tree and file preview modes.
- `../modes.rs` 内联 `fuzzy_picker`: fuzzy picker input.
- `agent_launcher.rs` / `agent_launcher_tests.rs`: launch selected agents in native PTY tabs, register live sidebar entries, retain explicit tmux compatibility routing, and apply Codex/Claude runtime wiring.
- `../modes.rs` 内联 `agent_style`: agent style selector key handling.
- `../modes.rs` 内联 `delete_confirm` / `../modes.rs` 内联 `help` / `telegram.rs`: delete confirmation, help, and Telegram modal keys.
- `notification_inbox.rs` / `notification_inbox_tests.rs`: notification inbox navigation and action keys.

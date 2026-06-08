# notification_inbox

- `mod.rs`: notification inbox public API.
- `model.rs` / `model/`: persisted inbox data model, entry shape, mutation helpers, and display time helpers.
- `storage.rs` / `storage_tests.rs`: JSON load/save/mutation helpers for `~/.pad/notifications/inbox.json`.

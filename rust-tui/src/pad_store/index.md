# pad_store

- `mod.rs`: PAD-only database opening, path guard, error types, and provider-independent model/UI-state exports.
- `schema.rs`: SQLite schema, foreign-key triggers, and sequential migrations; v2 adds the bounded PAD-only `desktop_ui_state` singleton.
- `repository.rs`: Profile, Project, Task, Section, and SectionItem CRUD plus a strongly typed Desktop UI state API (opaque IDs, 240–520 sidebar width, fixed theme enum, bounded collapse lists).
- `repository/support.rs`: SQL row mapping, section-write helpers, timestamps, enums, and bounded Desktop UI-state validation shared by the repository API.
- `tests.rs`: schema, CRUD, restart, v1→v2 record preservation, UI-state constraints/corruption/isolation, referential-integrity, deletion, and provider-path tests.

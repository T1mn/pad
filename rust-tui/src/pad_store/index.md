# pad_store

- `mod.rs`: PAD-only database opening, path guard, error types, and provider-independent model exports.
- `schema.rs`: SQLite schema, foreign-key triggers, and versioned migrations.
- `repository.rs`: Profile, Project, Task, Section, and SectionItem CRUD repository.
- `tests.rs`: schema, CRUD, restart, referential-integrity, deletion, and provider-path tests.

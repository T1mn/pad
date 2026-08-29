use super::*;
use crate::permission_policy::{PolicyLayer, ProtectedNamespace};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

struct TemporaryDatabase {
    path: PathBuf,
}

impl TemporaryDatabase {
    fn new() -> Self {
        let sequence = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before Unix epoch")
            .as_nanos();
        Self {
            path: std::env::temp_dir().join(format!(
                "pad-private-store-{}-{sequence}-{stamp}.sqlite",
                std::process::id()
            )),
        }
    }
}

impl Drop for TemporaryDatabase {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_file(self.path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(self.path.with_extension("sqlite-shm"));
    }
}

fn profile(id: &str) -> Profile {
    Profile {
        id: id.to_string(),
        name: format!("Profile {id}"),
        agent_dir: PathBuf::from(format!("/tmp/pad-agent-{id}")),
        session_dir: PathBuf::from(format!("/tmp/pad-sessions-{id}")),
        credential_ref: Some(format!("keychain://pad/{id}")),
        default_provider: Some("anthropic".to_string()),
        default_model: Some("claude-sonnet".to_string()),
        policy: PolicyLayer {
            workspace_roots: vec![PathBuf::from("/tmp/workspace")],
            protected_namespaces: vec![ProtectedNamespace::new("pad-private", "/tmp/pad-private")],
            ..PolicyLayer::default()
        },
        created_at: 100,
        updated_at: 100,
    }
}

fn project(id: &str, profile_id: Option<&str>) -> Project {
    Project {
        id: id.to_string(),
        name: format!("Project {id}"),
        primary_root: PathBuf::from(format!("/tmp/project-{id}")),
        additional_roots: vec![PathBuf::from(format!("/tmp/project-{id}/docs"))],
        profile_id: profile_id.map(str::to_string),
        policy: PolicyLayer::default(),
        pinned: false,
        archived: false,
        created_at: 200,
        updated_at: 200,
    }
}

fn task(id: &str, project_id: Option<&str>, profile_id: &str) -> Task {
    Task {
        id: id.to_string(),
        project_id: project_id.map(str::to_string),
        profile_id: profile_id.to_string(),
        pi_session_id: Some(format!("pi-session-{id}")),
        session_file: Some(PathBuf::from(format!("/tmp/pi-sessions/{id}.jsonl"))),
        title: format!("Task {id}"),
        summary: "Persisted task summary".to_string(),
        cwd: PathBuf::from(format!("/tmp/project-{id}")),
        environment: TaskEnvironment::Local,
        status: TaskStatus::Running,
        leaf_id: Some("leaf-1".to_string()),
        unread: true,
        pinned: true,
        archived: false,
        policy: PolicyLayer::default(),
        created_at: 300,
        updated_at: 300,
    }
}

fn section(id: &str, items: Vec<SectionItem>) -> Section {
    Section {
        id: id.to_string(),
        name: format!("Section {id}"),
        order: 2,
        collapsed: false,
        items,
        created_at: 400,
        updated_at: 400,
    }
}

fn desktop_ui_state() -> DesktopUiState {
    DesktopUiState {
        active_profile_id: Some("profile-desktop".to_string()),
        selected_task_id: Some("task-desktop".to_string()),
        collapsed_section_ids: vec!["section-pinned".to_string(), "section-recent".to_string()],
        collapsed_project_ids: vec!["project-alpha".to_string()],
        sidebar_width: SidebarWidth::new(318).unwrap(),
        sidebar_view: super::DesktopSidebarView::Pinned,
        theme: DesktopTheme::Dark,
        right_panel_open: true,
        bottom_panel_open: true,
        sidebar_open: true,
    }
}

pub(crate) fn builds_private_schema_with_foreign_keys_and_version() {
    let database = TemporaryDatabase::new();
    let store = PadStore::open(&database.path).expect("open PAD store");

    assert_eq!(
        store.db_path(),
        Some(validate_database_path(&database.path).unwrap().as_path())
    );
    let tables: Vec<String> = {
        let mut statement = store
            .connection()
            .prepare(
                "SELECT name FROM sqlite_master
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap()
    };
    assert_eq!(
        tables,
        vec![
            "desktop_ui_state",
            "profiles",
            "projects",
            "section_items",
            "sections",
            "tasks"
        ]
    );

    let foreign_keys: i64 = store
        .connection()
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .unwrap();
    let version: i64 = store
        .connection()
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(foreign_keys, 1);
    assert_eq!(version, CURRENT_SCHEMA_VERSION);
}

pub(crate) fn desktop_ui_state_round_trip_survives_close_and_reopen() {
    let database = TemporaryDatabase::new();
    let state = desktop_ui_state();

    {
        let mut store = PadStore::open(&database.path).unwrap();
        assert_eq!(
            store.get_desktop_ui_state().unwrap(),
            DesktopUiState::default()
        );
        store.set_desktop_ui_state(&state).unwrap();
        assert_eq!(store.get_desktop_ui_state().unwrap(), state);
    }

    let store = PadStore::open(&database.path).unwrap();
    let reopened = store.get_desktop_ui_state().unwrap();
    assert_eq!(reopened, state);
    assert_eq!(reopened.sidebar_width.get(), 318);
    assert_eq!(reopened.theme, DesktopTheme::Dark);
}

pub(crate) fn migration_from_v1_preserves_all_existing_records() {
    let database = TemporaryDatabase::new();
    let expected_profile = profile("v1-profile");
    let expected_project = project("v1-project", Some("v1-profile"));
    let expected_task = task("v1-task", Some("v1-project"), "v1-profile");
    let expected_section = section(
        "v1-section",
        vec![
            SectionItem::Project("v1-project".to_string()),
            SectionItem::Task("v1-task".to_string()),
        ],
    );

    {
        let mut store = PadStore::open(&database.path).unwrap();
        store.insert_profile(&expected_profile).unwrap();
        store.insert_project(&expected_project).unwrap();
        store.insert_task(&expected_task).unwrap();
        store.insert_section(&expected_section).unwrap();

        // Reconstruct the exact version boundary: v1 has all metadata tables
        // and records, but no Desktop UI state singleton table.
        store
            .connection()
            .execute_batch("DROP TABLE desktop_ui_state; PRAGMA user_version = 1;")
            .unwrap();
    }

    let store = PadStore::open(&database.path).unwrap();
    assert_eq!(
        store.get_profile("v1-profile").unwrap(),
        Some(expected_profile)
    );
    assert_eq!(
        store.get_project("v1-project").unwrap(),
        Some(expected_project)
    );
    assert_eq!(store.get_task("v1-task").unwrap(), Some(expected_task));
    assert_eq!(
        store.get_section("v1-section").unwrap(),
        Some(expected_section)
    );
    assert_eq!(
        store.get_desktop_ui_state().unwrap(),
        DesktopUiState::default()
    );
    let version: i64 = store
        .connection()
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, CURRENT_SCHEMA_VERSION);
}

pub(crate) fn desktop_ui_state_enforces_bounds_and_reports_corruption() {
    assert!(SidebarWidth::new(239).is_err());
    assert!(SidebarWidth::new(240).is_ok());
    assert!(SidebarWidth::new(520).is_ok());
    assert!(SidebarWidth::new(521).is_err());

    let database = TemporaryDatabase::new();
    let mut store = PadStore::open(&database.path).unwrap();
    let mut invalid_state = desktop_ui_state();
    invalid_state.collapsed_project_ids = vec!["duplicate".to_string(); 2];
    assert!(matches!(
        store.set_desktop_ui_state(&invalid_state),
        Err(StoreError::Serialization(_))
    ));

    invalid_state = desktop_ui_state();
    invalid_state.selected_task_id = Some("/Users/example/private/task".to_string());
    assert!(matches!(
        store.set_desktop_ui_state(&invalid_state),
        Err(StoreError::Serialization(_))
    ));

    invalid_state = desktop_ui_state();
    invalid_state.collapsed_section_ids = (0..=256).map(|index| format!("s-{index}")).collect();
    assert!(matches!(
        store.set_desktop_ui_state(&invalid_state),
        Err(StoreError::Serialization(_))
    ));

    store.set_desktop_ui_state(&desktop_ui_state()).unwrap();
    let invalid_width = store.connection().execute(
        "UPDATE desktop_ui_state
         SET state_json = json_set(state_json, '$.sidebar_width', 521)
         WHERE singleton_id = 1",
        [],
    );
    assert!(invalid_width.is_err());
    let missing_required_fields = store.connection().execute(
        "UPDATE desktop_ui_state SET state_json = '{}' WHERE singleton_id = 1",
        [],
    );
    assert!(missing_required_fields.is_err());

    // Simulate on-disk damage after a crash or external modification. Reads
    // must report it and must never silently overwrite it with defaults.
    store
        .connection()
        .execute_batch(
            "PRAGMA ignore_check_constraints = ON;
             UPDATE desktop_ui_state SET state_json = '{not-json' WHERE singleton_id = 1;
             PRAGMA ignore_check_constraints = OFF;",
        )
        .unwrap();
    let error = store.get_desktop_ui_state().unwrap_err();
    assert!(matches!(error, StoreError::Serialization(_)));
    assert!(error.to_string().contains("stored state_json is corrupt"));
}

pub(crate) fn desktop_ui_state_document_is_pad_private_and_secret_free() {
    let database = TemporaryDatabase::new();
    let mut store = PadStore::open(&database.path).unwrap();
    let mut private_profile = profile("profile-desktop");
    private_profile.agent_dir = PathBuf::from("/Users/example/.pad/private-agent");
    private_profile.session_dir = PathBuf::from("/Users/example/.pad/private-sessions");
    private_profile.credential_ref = Some("keychain://pad/credential-secret".to_string());
    store.insert_profile(&private_profile).unwrap();
    store.set_desktop_ui_state(&desktop_ui_state()).unwrap();

    let encoded: String = store
        .connection()
        .query_row(
            "SELECT state_json FROM desktop_ui_state WHERE singleton_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let object = serde_json::from_str::<serde_json::Value>(&encoded)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    let mut keys = object.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "active_profile_id",
            "bottom_panel_open",
            "collapsed_project_ids",
            "collapsed_section_ids",
            "right_panel_open",
            "selected_task_id",
            "sidebar_open",
            "sidebar_view",
            "sidebar_width",
            "theme",
        ]
    );
    for forbidden in [
        "credential_ref",
        "credential-secret",
        "agent_dir",
        "session_dir",
        "keychain://",
        "/Users/example",
        "cwd",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "leaked {forbidden}: {encoded}"
        );
    }
}

pub(crate) fn crud_round_trip_preserves_profile_project_task_and_section_items() {
    let database = TemporaryDatabase::new();
    let mut store = PadStore::open(&database.path).unwrap();
    let mut stored_profile = profile("p1");
    let mut stored_project = project("project-1", Some("p1"));
    let stored_task = task("task-1", Some("project-1"), "p1");

    store.insert_profile(&stored_profile).unwrap();
    store.insert_project(&stored_project).unwrap();
    store.insert_task(&stored_task).unwrap();
    store
        .insert_section(&section(
            "section-1",
            vec![
                SectionItem::Project("project-1".to_string()),
                SectionItem::Task("task-1".to_string()),
            ],
        ))
        .unwrap();

    assert_eq!(
        store.get_profile("p1").unwrap(),
        Some(stored_profile.clone())
    );
    assert_eq!(
        store.get_project("project-1").unwrap(),
        Some(stored_project.clone())
    );
    assert_eq!(store.get_task("task-1").unwrap(), Some(stored_task.clone()));
    assert_eq!(
        store.section_items("section-1").unwrap(),
        vec![
            SectionItem::Project("project-1".to_string()),
            SectionItem::Task("task-1".to_string()),
        ]
    );

    stored_profile.name = "Updated profile".to_string();
    stored_profile.updated_at = 101;
    store.update_profile(&stored_profile).unwrap();
    stored_project.pinned = true;
    stored_project.archived = true;
    stored_project.updated_at = 201;
    store.update_project(&stored_project).unwrap();
    assert_eq!(store.get_profile("p1").unwrap(), Some(stored_profile));
    assert_eq!(
        store.get_project("project-1").unwrap(),
        Some(stored_project)
    );

    store
        .replace_section_items("section-1", &[SectionItem::Task("task-1".to_string())])
        .unwrap();
    assert_eq!(
        store.section_items("section-1").unwrap(),
        vec![SectionItem::Task("task-1".to_string())]
    );
    store
        .add_section_item("section-1", &SectionItem::Project("project-1".to_string()))
        .unwrap();
    store
        .remove_section_item("section-1", &SectionItem::Task("task-1".to_string()))
        .unwrap();
    assert_eq!(
        store.section_items("section-1").unwrap(),
        vec![SectionItem::Project("project-1".to_string())]
    );
}

pub(crate) fn data_survives_close_and_reopen() {
    let database = TemporaryDatabase::new();
    {
        let mut store = PadStore::open(&database.path).unwrap();
        store.insert_profile(&profile("restart-profile")).unwrap();
        store
            .insert_project(&project("restart-project", Some("restart-profile")))
            .unwrap();
        store
            .insert_task(&task(
                "restart-task",
                Some("restart-project"),
                "restart-profile",
            ))
            .unwrap();
    }

    let store = PadStore::open(&database.path).unwrap();
    assert_eq!(store.list_profiles().unwrap().len(), 1);
    assert_eq!(store.list_projects(false).unwrap().len(), 1);
    assert_eq!(
        store
            .list_tasks(Some("restart-project"), false)
            .unwrap()
            .len(),
        1
    );
    assert!(store.get_task("restart-task").unwrap().unwrap().unread);
    let (profiles, projects, tasks, sections) = store.load_sidebar_records().unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(projects.len(), 1);
    assert_eq!(tasks.len(), 1);
    assert!(sections.is_empty());
}

pub(crate) fn foreign_keys_and_polymorphic_triggers_reject_orphan_references() {
    let database = TemporaryDatabase::new();
    let mut store = PadStore::open(&database.path).unwrap();
    store.insert_profile(&profile("p1")).unwrap();
    store
        .insert_section(&section("section-1", Vec::new()))
        .unwrap();

    assert!(store
        .insert_task(&task("missing-profile", None, "does-not-exist"))
        .is_err());
    assert!(store
        .add_section_item(
            "section-1",
            &SectionItem::Project("does-not-exist".to_string()),
        )
        .is_err());
    assert!(store
        .add_section_item(
            "section-1",
            &SectionItem::Task("does-not-exist".to_string()),
        )
        .is_err());
    assert!(store.section_items("section-1").unwrap().is_empty());

    // The SQL trigger remains active even for a direct connection caller.
    let direct_insert = store.connection().execute(
        "INSERT INTO section_items(section_id, item_kind, item_id, item_order)
         VALUES ('section-1', 'project', 'does-not-exist', 0)",
        [],
    );
    assert!(direct_insert.is_err());
}

pub(crate) fn deleting_targets_removes_section_items_without_orphans() {
    let database = TemporaryDatabase::new();
    let mut store = PadStore::open(&database.path).unwrap();
    store.insert_profile(&profile("p1")).unwrap();
    store
        .insert_project(&project("project-1", Some("p1")))
        .unwrap();
    store
        .insert_task(&task("task-1", Some("project-1"), "p1"))
        .unwrap();
    store
        .insert_section(&section(
            "section-1",
            vec![
                SectionItem::Project("project-1".to_string()),
                SectionItem::Task("task-1".to_string()),
            ],
        ))
        .unwrap();

    store.delete_task("task-1").unwrap();
    assert_eq!(
        store.section_items("section-1").unwrap(),
        vec![SectionItem::Project("project-1".to_string())]
    );
    store.delete_project("project-1").unwrap();
    assert!(store.section_items("section-1").unwrap().is_empty());
}

pub(crate) fn provider_owned_paths_are_rejected_before_database_creation() {
    let root = TemporaryDatabase::new();
    let protected = root
        .path
        .parent()
        .unwrap()
        .join(".codex")
        .join("pad.sqlite");
    assert!(matches!(
        PadStore::open(&protected),
        Err(StoreError::InvalidDatabasePath(_))
    ));
    assert!(!protected.exists());
}

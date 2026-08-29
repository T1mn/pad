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

#[test]
fn builds_private_schema_with_foreign_keys_and_version() {
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
        vec!["profiles", "projects", "section_items", "sections", "tasks"]
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

#[test]
fn crud_round_trip_preserves_profile_project_task_and_section_items() {
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

#[test]
fn data_survives_close_and_reopen() {
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
    assert_eq!(
        store.get_task("restart-task").unwrap().unwrap().unread,
        true
    );
    let (profiles, projects, tasks, sections) = store.load_sidebar_records().unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(projects.len(), 1);
    assert_eq!(tasks.len(), 1);
    assert!(sections.is_empty());
}

#[test]
fn foreign_keys_and_polymorphic_triggers_reject_orphan_references() {
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

#[test]
fn deleting_targets_removes_section_items_without_orphans() {
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

#[test]
fn provider_owned_paths_are_rejected_before_database_creation() {
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

use super::permission_policy::*;
use std::path::{Path, PathBuf};

fn workspace_policy(mode: PermissionMode) -> EffectivePolicy {
    EffectivePolicy {
        mode,
        workspace_roots: vec![PathBuf::from("/work/project")],
        protected_namespaces: vec![ProtectedNamespace::new("codex-home", "/Users/test/.codex")],
        unattended: false,
    }
}

#[test]
fn permission_modes_use_stable_snake_case_json() {
    assert_eq!(
        serde_json::to_string(&PermissionMode::Guarded).unwrap(),
        "\"guarded\""
    );
    assert_eq!(
        serde_json::to_string(&PermissionMode::WorkspaceFull).unwrap(),
        "\"workspace_full\""
    );
    assert_eq!(
        serde_json::to_string(&PermissionMode::SystemFull).unwrap(),
        "\"system_full\""
    );
}

#[test]
fn lexical_canonicalization_resolves_traversal() {
    assert_eq!(
        canonicalize_policy_path(
            Path::new("./src/../.codex/session.json"),
            Path::new("/work")
        ),
        PathBuf::from("/work/.codex/session.json")
    );
    assert_eq!(
        canonicalize_policy_path(Path::new("../../etc"), Path::new("/work/project")),
        PathBuf::from("/etc")
    );
}

#[test]
fn protected_namespace_matching_is_component_safe() {
    let namespaces = vec![ProtectedNamespace::new("codex", "/Users/test/.codex")];
    assert!(matching_protected_namespace(
        Path::new("/Users/test/.codex/sessions/a.jsonl"),
        Path::new("/"),
        &namespaces
    )
    .is_some());
    assert!(matching_protected_namespace(
        Path::new("/Users/test/.codex-backup/a"),
        Path::new("/"),
        &namespaces
    )
    .is_none());
}

#[test]
fn guarded_allows_reads_and_prompts_for_writes() {
    let policy = workspace_policy(PermissionMode::Guarded);
    let read = evaluate_operation(
        &policy,
        &PolicyOperation::at_path(OperationKind::Read, "/work/project/src/main.rs"),
        Path::new("/work/project"),
    );
    assert!(read.is_allowed());
    assert_eq!(read.risk(), RiskClass::ReadOnly);

    let write = evaluate_operation(
        &policy,
        &PolicyOperation::at_path(OperationKind::Write, "/work/project/src/main.rs"),
        Path::new("/work/project"),
    );
    assert!(write.requires_confirmation());
    assert_eq!(write.risk(), RiskClass::WorkspaceWrite);
}

#[test]
fn workspace_full_allows_workspace_mutation_but_prompts_external_work() {
    let policy = workspace_policy(PermissionMode::WorkspaceFull);
    let delete = evaluate_operation(
        &policy,
        &PolicyOperation::at_path(OperationKind::Delete, "/work/project/tmp"),
        Path::new("/work/project"),
    );
    assert!(delete.is_allowed());
    assert_eq!(delete.risk(), RiskClass::WorkspaceDestructive);

    let network = evaluate_operation(
        &policy,
        &PolicyOperation::new(OperationKind::Network),
        Path::new("/work/project"),
    );
    assert!(network.requires_confirmation());

    let outside = evaluate_operation(
        &policy,
        &PolicyOperation::at_path(OperationKind::Write, "/tmp/outside"),
        Path::new("/work/project"),
    );
    assert!(outside.requires_confirmation());
    assert_eq!(outside.risk(), RiskClass::ExternalWrite);
}

#[test]
fn system_full_allows_external_operations_but_never_protected_namespace() {
    let policy = workspace_policy(PermissionMode::SystemFull);
    let network = evaluate_operation(
        &policy,
        &PolicyOperation::new(OperationKind::Network),
        Path::new("/work/project"),
    );
    assert!(network.is_allowed());

    let protected = evaluate_operation(
        &policy,
        &PolicyOperation::at_path(
            OperationKind::Delete,
            "/Users/test/.codex/sessions/current.jsonl",
        ),
        Path::new("/work/project"),
    );
    assert!(protected.is_denied());
    assert_eq!(protected.risk(), RiskClass::ProtectedNamespace);
}

#[test]
fn unattended_policy_turns_confirmation_into_deny() {
    let mut policy = workspace_policy(PermissionMode::Guarded);
    policy.unattended = true;
    let decision = evaluate_operation(
        &policy,
        &PolicyOperation::at_path(OperationKind::Write, "/work/project/file"),
        Path::new("/work/project"),
    );
    assert!(decision.is_denied());
    assert!(!decision.requires_confirmation());
}

#[test]
fn merge_layers_uses_specific_scalar_and_additive_collections() {
    let profile = PolicyLayer {
        mode: Some(PermissionMode::Guarded),
        workspace_roots: vec![PathBuf::from("/profile")],
        protected_namespaces: vec![ProtectedNamespace::new("profile", "/profile/private")],
        unattended: Some(false),
    };
    let project = PolicyLayer {
        mode: Some(PermissionMode::WorkspaceFull),
        workspace_roots: vec![PathBuf::from("/project")],
        protected_namespaces: vec![ProtectedNamespace::new("project", "/project/private")],
        unattended: None,
    };
    let task = PolicyLayer {
        mode: Some(PermissionMode::SystemFull),
        workspace_roots: vec![PathBuf::from("/task")],
        protected_namespaces: vec![ProtectedNamespace::new("task", "/task/private")],
        unattended: Some(true),
    };
    let merged = merge_policy_layers(&profile, Some(&project), Some(&task));
    assert_eq!(merged.mode, PermissionMode::SystemFull);
    assert!(merged.unattended);
    assert_eq!(merged.workspace_roots.len(), 3);
    assert_eq!(merged.protected_namespaces.len(), 3);
}

#[test]
fn model_hierarchy_adds_project_roots_and_task_cwd() {
    let profile = Profile {
        id: "p".into(),
        name: "default".into(),
        agent_dir: "/data/p".into(),
        session_dir: "/data/p/sessions".into(),
        ..Profile::default()
    };
    let project = Project {
        id: "project".into(),
        name: "PAD".into(),
        primary_root: "/work/project".into(),
        additional_roots: vec![PathBuf::from("/work/shared")],
        ..Project::default()
    };
    let task = Task {
        id: "task".into(),
        profile_id: "p".into(),
        cwd: "/work/project/src".into(),
        ..Task::default()
    };
    let effective = merge_profile_project_task(&profile, Some(&project), Some(&task));
    assert_eq!(effective.workspace_roots.len(), 3);
    assert!(effective
        .protected_namespaces
        .iter()
        .any(|item| item.name == "profile-agent-dir"));
}

#[test]
fn defaults_protect_pad_pi_and_codex_namespaces() {
    let namespaces = default_protected_namespaces(Path::new("/Users/test"));
    assert!(namespaces
        .iter()
        .any(|item| item.root == Path::new("/Users/test/.codex")));
    assert!(namespaces
        .iter()
        .any(|item| item.root == Path::new("/Users/test/.pi")));
    assert!(namespaces
        .iter()
        .any(|item| item.root == Path::new("/Users/test/.pad")));
}

#[test]
fn pi_session_metadata_round_trips_parent_and_cursor_fields() {
    let header = PiSessionHeader {
        entry_type: "session".into(),
        version: 3,
        id: "session-1".into(),
        timestamp: Some("2026-08-29T00:00:00Z".into()),
        cwd: "/work/project".into(),
        parent_session: Some("parent-1".into()),
    };
    let value = serde_json::to_value(&header).unwrap();
    assert_eq!(value["parentSession"], "parent-1");
    assert_eq!(
        serde_json::from_value::<PiSessionHeader>(value).unwrap(),
        header
    );

    let cursor = PiSessionCursor {
        last_entry_id: Some("entry-4".into()),
        leaf_id: Some("entry-4".into()),
        rpc_sequence: 7,
    };
    assert_eq!(
        serde_json::from_str::<PiSessionCursor>(&serde_json::to_string(&cursor).unwrap()).unwrap(),
        cursor
    );
}

#[test]
fn sidebar_section_serializes_polymorphic_items() {
    let section = Section {
        id: "sec".into(),
        name: "Pinned Work".into(),
        order: 1,
        collapsed: false,
        items: vec![
            SectionItem::Project("project".into()),
            SectionItem::Task("task".into()),
        ],
        created_at: 1,
        updated_at: 1,
    };
    let json = serde_json::to_value(&section).unwrap();
    assert_eq!(json["items"][0]["kind"], "project");
    assert_eq!(json["items"][1]["id"], "task");
}

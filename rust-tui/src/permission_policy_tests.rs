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

pub(crate) fn permission_modes_use_stable_snake_case_json() {
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

pub(crate) fn lexical_canonicalization_resolves_traversal() {
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

pub(crate) fn protected_namespace_matching_is_component_safe() {
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

pub(crate) fn guarded_allows_reads_and_prompts_for_writes() {
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

pub(crate) fn workspace_full_allows_workspace_mutation_but_prompts_external_work() {
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

pub(crate) fn system_full_allows_external_operations_but_never_protected_namespace() {
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

    let credential = evaluate_operation(
        &policy,
        &PolicyOperation::new(OperationKind::Credential),
        Path::new("/work/project"),
    );
    assert!(credential.is_denied());
    assert_eq!(credential.risk(), RiskClass::Credential);

    let dynamic_credential = evaluate_operation(
        &policy,
        &PolicyOperation {
            kind: OperationKind::Credential,
            path: None,
            command: Some("security find-generic-password > $OUTPUT".into()),
        },
        Path::new("/work/project"),
    );
    assert!(dynamic_credential.is_denied());
    assert_eq!(dynamic_credential.risk(), RiskClass::Credential);

    let protected_command = evaluate_operation(
        &policy,
        &PolicyOperation {
            kind: OperationKind::Execute,
            path: None,
            command: Some("rm -rf ~/.codex/sessions".into()),
        },
        Path::new("/work/project"),
    );
    assert!(protected_command.is_denied());
    assert_eq!(protected_command.risk(), RiskClass::ProtectedNamespace);
}

pub(crate) fn system_full_only_auto_allows_statically_verified_shell_commands() {
    let mut policy = workspace_policy(PermissionMode::SystemFull);
    policy.unattended = true;
    policy.protected_namespaces = default_protected_namespaces(Path::new("/Users/test"));

    for (command, expected_risk) in [
        (
            r#"d=.cod; rm -rf "$HOME/${d}ex/sessions""#,
            RiskClass::Unknown,
        ),
        (
            r#"rm -rf "$HOME/.codex/sessions""#,
            RiskClass::ProtectedNamespace,
        ),
        (
            r#"rm -rf "$(printf '/Users/test/.codex/sessions')""#,
            RiskClass::ProtectedNamespace,
        ),
        ("printf secret > /tmp/pad-policy-output", RiskClass::Unknown),
    ] {
        let decision = evaluate_operation(
            &policy,
            &PolicyOperation {
                kind: OperationKind::Execute,
                path: None,
                command: Some(command.into()),
            },
            Path::new("/work/project"),
        );
        assert!(
            decision.is_denied(),
            "dynamic command was allowed: {command}"
        );
        assert_eq!(decision.risk(), expected_risk);
    }

    for command in ["git status --short", "ls -la /tmp", "rm -f /tmp/pad-safe"] {
        let decision = evaluate_operation(
            &policy,
            &PolicyOperation {
                kind: OperationKind::Execute,
                path: None,
                command: Some(command.into()),
            },
            Path::new("/work/project"),
        );
        assert!(
            decision.is_allowed(),
            "literal ordinary command was not allowed: {command}"
        );
    }

    policy.unattended = false;
    let attended = evaluate_operation(
        &policy,
        &PolicyOperation {
            kind: OperationKind::Execute,
            path: None,
            command: Some("echo value > /tmp/pad-policy-output".into()),
        },
        Path::new("/work/project"),
    );
    assert!(attended.requires_confirmation());
    assert_eq!(attended.risk(), RiskClass::Unknown);
}

pub(crate) fn quoted_and_concatenated_shell_literals_cannot_hide_protected_paths() {
    let mut policy = workspace_policy(PermissionMode::SystemFull);
    policy.unattended = true;
    policy.protected_namespaces = default_protected_namespaces(Path::new("/Users/test"));

    for command in [
        "rm -rf '/Users/test/.codex/sessions'",
        "rm -rf '/Users/test/.co''dex/sessions'",
        "rm -rf \"/Users/test/Library/Application Support/PAD Desktop/v1\"",
    ] {
        let decision = evaluate_operation(
            &policy,
            &PolicyOperation {
                kind: OperationKind::Execute,
                path: None,
                command: Some(command.into()),
            },
            Path::new("/work/project"),
        );
        assert!(
            decision.is_denied(),
            "protected command was allowed: {command}"
        );
        assert_eq!(decision.risk(), RiskClass::ProtectedNamespace);
    }
}

pub(crate) fn symlink_targets_are_resolved_before_full_access_is_allowed() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let root = crate::test_support::temp_path("pad-policy", "symlink-protected");
        let workspace = root.join("workspace");
        let protected = root.join("provider-state");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(protected.join("sessions")).unwrap();
        let link = workspace.join("provider-link");
        symlink(&protected, &link).unwrap();

        let policy = EffectivePolicy {
            mode: PermissionMode::SystemFull,
            workspace_roots: vec![workspace.clone()],
            protected_namespaces: vec![ProtectedNamespace::new(
                "provider-state",
                protected.clone(),
            )],
            unattended: true,
        };
        let path_decision = evaluate_operation(
            &policy,
            &PolicyOperation::at_path(OperationKind::Delete, link.join("sessions")),
            &workspace,
        );
        assert!(path_decision.is_denied());
        assert_eq!(path_decision.risk(), RiskClass::ProtectedNamespace);

        for command in [
            format!("rm -rf '{}'", link.join("sessions").display()),
            "rm -rf 'provider-link/sessions'".to_string(),
        ] {
            let command_decision = evaluate_operation(
                &policy,
                &PolicyOperation {
                    kind: OperationKind::Execute,
                    path: None,
                    command: Some(command),
                },
                &workspace,
            );
            assert!(command_decision.is_denied());
            assert_eq!(command_decision.risk(), RiskClass::ProtectedNamespace);
        }

        std::fs::remove_dir_all(&root).unwrap();
    }
}

pub(crate) fn unattended_policy_turns_confirmation_into_deny() {
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

pub(crate) fn merge_layers_uses_specific_scalar_and_additive_collections() {
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

pub(crate) fn model_hierarchy_adds_project_roots_and_task_cwd() {
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

pub(crate) fn defaults_protect_pad_pi_and_codex_namespaces() {
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
    assert!(namespaces
        .iter()
        .any(|item| { item.root == Path::new("/Users/test/Library/Containers/com.openai.chat") }));
    for expected in [
        "/Users/test/Library/Application Support/Codex",
        "/Users/test/Library/Application Support/OpenAI",
        "/Users/test/Library/Application Support/ChatGPT",
        "/Users/test/Library/Group Containers/2DC432GLL2.com.openai.codex.notifications",
        "/Users/test/Library/Group Containers/2DC432GLL2.com.openai.sky.CUAService",
        "/Users/test/Library/Caches/Codex",
        "/Users/test/Library/Caches/com.openai.codex",
        "/Users/test/Library/Logs/com.openai.codex",
        "/Users/test/Library/HTTPStorages/com.openai.codex",
        "/Users/test/Library/HTTPStorages/com.openai.codex.binarycookies",
        "/Users/test/Library/Preferences/com.openai.codex.plist",
    ] {
        assert!(
            namespaces
                .iter()
                .any(|item| item.root == Path::new(expected)),
            "missing default protected namespace: {expected}"
        );
    }
    assert!(namespaces.iter().any(|item| {
        item.root == Path::new("/Users/test/Library/Application Support/PAD Desktop")
    }));
}

pub(crate) fn pi_session_metadata_round_trips_parent_and_cursor_fields() {
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

pub(crate) fn sidebar_section_serializes_polymorphic_items() {
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

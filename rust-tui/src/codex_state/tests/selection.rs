use super::super::pathing::select_latest_thread_for_cwd;
use std::path::Path;

#[test]
fn prefers_exact_cwd_match_before_related_threads() {
    let threads = vec![
        super::super::CodexThreadRef {
            thread_id: "older-exact".into(),
            cwd: "/tmp/project".into(),
            updated_at: 100,
            rollout_path: "/tmp/a.jsonl".into(),
            title: None,
            first_user_message: None,
            source: None,
            archived: false,
        },
        super::super::CodexThreadRef {
            thread_id: "newer-parent".into(),
            cwd: "/tmp".into(),
            updated_at: 999,
            rollout_path: "/tmp/b.jsonl".into(),
            title: None,
            first_user_message: None,
            source: None,
            archived: false,
        },
    ];

    let selected = select_latest_thread_for_cwd(Path::new("/tmp/project"), &threads).unwrap();
    assert_eq!(selected.thread_id, "older-exact");
}

#[test]
fn falls_back_to_closest_related_thread_when_exact_match_missing() {
    let threads = vec![
        super::super::CodexThreadRef {
            thread_id: "generic-parent".into(),
            cwd: "/tmp".into(),
            updated_at: 999,
            rollout_path: "/tmp/a.jsonl".into(),
            title: None,
            first_user_message: None,
            source: None,
            archived: false,
        },
        super::super::CodexThreadRef {
            thread_id: "project-parent".into(),
            cwd: "/tmp/project".into(),
            updated_at: 200,
            rollout_path: "/tmp/b.jsonl".into(),
            title: None,
            first_user_message: None,
            source: None,
            archived: false,
        },
    ];

    let selected =
        select_latest_thread_for_cwd(Path::new("/tmp/project/subdir"), &threads).unwrap();
    assert_eq!(selected.thread_id, "project-parent");
}

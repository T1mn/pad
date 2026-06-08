use super::{read_status, write_status_body, ProcessStatus, StatusGuard};
use std::fs;

#[test]
fn status_guard_drop_preserves_newer_status_file() {
    let path = std::env::temp_dir().join(format!(
        "pad-status-guard-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let guard = StatusGuard::new(path.clone(), "telegram-bot").unwrap();
    write_status_body(
        &path,
        &ProcessStatus {
            pid: guard.pid.saturating_add(1),
            started_at: guard.started_at.saturating_add(1),
            mode: "telegram-bot".to_string(),
        },
    )
    .unwrap();
    drop(guard);

    let status = read_status(&path).unwrap();
    assert_eq!(status.pid, std::process::id().saturating_add(1));

    let _ = fs::remove_file(path);
}

#[test]
fn stat_parser_treats_zombies_as_not_alive() {
    assert!(super::process::stat_indicates_zombie("Z+"));
    assert!(super::process::stat_indicates_zombie("SZ"));
    assert!(!super::process::stat_indicates_zombie("S+"));
    assert!(!super::process::stat_indicates_zombie("R"));
}

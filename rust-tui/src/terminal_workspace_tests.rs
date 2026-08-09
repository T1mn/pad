use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::TerminalProfile;

use super::*;

pub(crate) fn missing_workspace_is_not_an_error() {
    let directory = scratch_dir("missing");
    assert!(load_from_path(&directory.join("workspace.json"))
        .unwrap()
        .is_none());
}

pub(crate) fn workspace_round_trips_without_runtime_state() {
    let directory = scratch_dir("roundtrip");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("workspace.json");
    let mut workspace = TerminalWorkspace::default();
    workspace
        .add_tab(TerminalProfile::Shell, directory.clone())
        .unwrap();
    workspace
        .split_focused(
            crate::app::TerminalSplitAxis::Columns,
            TerminalProfile::Codex,
            directory.clone(),
        )
        .unwrap();

    save_to_path(&path, &workspace).unwrap();
    let restored = load_from_path(&path).unwrap().unwrap();

    assert_eq!(restored, workspace);
    let metadata = fs::metadata(&path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }
    fs::remove_dir_all(directory).unwrap();
}

pub(crate) fn invalid_or_future_workspace_is_rejected() {
    let directory = scratch_dir("invalid");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("workspace.json");
    fs::write(&path, "not json").unwrap();
    assert_eq!(
        load_from_path(&path).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    fs::write(&path, r#"{"version":999,"workspace":{}}"#).unwrap();
    assert_eq!(
        load_from_path(&path).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    fs::remove_dir_all(directory).unwrap();
}

pub(crate) fn invalid_workspace_is_quarantined_without_overwriting_recovery_files() {
    let directory = scratch_dir("quarantine");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("terminal-workspace.json");
    let first_recovery = directory.join("terminal-workspace.invalid.json");
    fs::write(&first_recovery, "older recovery").unwrap();
    fs::write(&path, "future workspace contents").unwrap();

    let quarantined = quarantine_invalid_at(&path).unwrap().unwrap();

    assert_eq!(
        quarantined,
        directory.join("terminal-workspace.invalid.1.json")
    );
    assert_eq!(
        fs::read_to_string(&quarantined).unwrap(),
        "future workspace contents"
    );
    assert_eq!(
        fs::read_to_string(&first_recovery).unwrap(),
        "older recovery"
    );
    assert!(!path.exists());
    fs::remove_dir_all(directory).unwrap();
}

fn scratch_dir(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "pad-terminal-workspace-{label}-{}-{nonce}",
        std::process::id()
    ))
}

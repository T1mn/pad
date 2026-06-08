use super::ProcessSnapshot;
use std::collections::HashMap;

#[test]
fn child_processes_joins_snapshot_commands_without_temp_vec() {
    let mut snapshot = ProcessSnapshot {
        loaded: true,
        snapshot_available: true,
        root_pids: vec!["10".to_string()],
        commands: HashMap::from([
            ("11".to_string(), "codex --resume abc".to_string()),
            ("12".to_string(), "claude".to_string()),
        ]),
        child_pids: HashMap::from([("10".to_string(), vec!["11".to_string(), "12".to_string()])]),
        full_commands: HashMap::new(),
    };

    assert_eq!(snapshot.child_processes("10"), "codex --resume abc claude");
}

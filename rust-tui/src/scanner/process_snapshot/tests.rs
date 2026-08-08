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

mod classify {
    use super::super::classify::{command_args_may_name_agent, command_may_hide_agent};

    #[test]
    fn distinguishes_shells_from_arg_wrappers() {
        assert!(command_may_hide_agent("/bin/zsh -l"));
        assert!(!command_args_may_name_agent("/bin/zsh -l"));
        assert!(command_may_hide_agent("node"));
        assert!(command_args_may_name_agent("node"));
    }

    #[test]
    fn wrapper_detection_is_ascii_case_insensitive_without_allocating() {
        assert!(command_may_hide_agent("/BIN/ZSH -l"));
        assert!(command_args_may_name_agent("/usr/local/bin/Node server.js"));
    }
}

mod loader {
    use super::super::loader::{normalize_root_pids, parse_process_snapshot};

    #[test]
    fn process_snapshot_parses_pid_ppid_and_args() {
        let (commands, children) = parse_process_snapshot(
            "  10     1 /bin/zsh -l\n  11    10 /opt/homebrew/bin/codex --resume abc\n",
            None,
        );

        assert_eq!(commands.get("10").map(String::as_str), Some("/bin/zsh -l"));
        assert_eq!(
            commands.get("11").map(String::as_str),
            Some("/opt/homebrew/bin/codex --resume abc")
        );
        assert_eq!(children.get("10"), Some(&vec!["11".to_string()]));
    }

    #[test]
    fn process_snapshot_filters_to_roots_and_direct_children() {
        let roots = normalize_root_pids(&["10".to_string()]);
        let (commands, children) = parse_process_snapshot(
            "  10     1 zsh\n  11    10 codex\n  12    11 node\n  20     1 unrelated\n",
            Some(&roots),
        );

        assert_eq!(commands.get("10").map(String::as_str), Some("zsh"));
        assert_eq!(commands.get("11").map(String::as_str), Some("codex"));
        assert!(!commands.contains_key("12"));
        assert!(!commands.contains_key("20"));
        assert_eq!(children.get("10"), Some(&vec!["11".to_string()]));
        assert!(!children.contains_key("11"));
    }
}

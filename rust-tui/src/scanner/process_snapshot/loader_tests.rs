use super::filter::normalize_root_pids;
use super::parse::parse_process_snapshot;

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

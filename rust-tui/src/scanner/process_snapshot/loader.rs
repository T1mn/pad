use std::collections::{HashMap, HashSet};
use std::process::Command;

pub(super) type ProcessMaps = (HashMap<String, String>, HashMap<String, Vec<String>>);

pub(super) fn load_lightweight_process_snapshot(root_pids: &[String]) -> Option<ProcessMaps> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,comm="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let roots = normalize_root_pids(root_pids);
    Some(parse_process_snapshot(
        &String::from_utf8_lossy(&output.stdout),
        (!roots.is_empty()).then_some(&roots),
    ))
}

pub(super) fn load_process_snapshot(root_pids: &[String]) -> Option<ProcessMaps> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,args="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let roots = normalize_root_pids(root_pids);
    Some(parse_process_snapshot(
        &String::from_utf8_lossy(&output.stdout),
        (!roots.is_empty()).then_some(&roots),
    ))
}

fn parse_process_snapshot(stdout: &str, root_filter: Option<&HashSet<String>>) -> ProcessMaps {
    let mut commands = HashMap::new();
    let mut child_pids: HashMap<String, Vec<String>> = HashMap::new();

    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let Some((pid, rest)) = split_once_whitespace(trimmed) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some((ppid, args)) = split_once_whitespace(rest) else {
            continue;
        };
        let args = args.trim_start();
        if args.is_empty() {
            continue;
        }

        let keep_command = should_keep_command(root_filter, pid, ppid);
        let keep_child = should_keep_child(root_filter, ppid);

        if keep_command {
            commands.insert(pid.to_string(), args.to_string());
        }
        if keep_child {
            child_pids
                .entry(ppid.to_string())
                .or_default()
                .push(pid.to_string());
        }
    }

    (commands, child_pids)
}

fn should_keep_command(root_filter: Option<&HashSet<String>>, pid: &str, ppid: &str) -> bool {
    match root_filter {
        Some(roots) => roots.contains(pid) || roots.contains(ppid),
        None => true,
    }
}

fn should_keep_child(root_filter: Option<&HashSet<String>>, ppid: &str) -> bool {
    match root_filter {
        Some(roots) => roots.contains(ppid),
        None => true,
    }
}

fn normalize_root_pids(root_pids: &[String]) -> HashSet<String> {
    root_pids
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|pid| !pid.is_empty())
        .map(str::to_string)
        .collect()
}

fn split_once_whitespace(value: &str) -> Option<(&str, &str)> {
    let split_at = value.find(char::is_whitespace)?;
    Some((&value[..split_at], &value[split_at..]))
}

#[cfg(test)]
mod tests {
    use super::{normalize_root_pids, parse_process_snapshot};

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

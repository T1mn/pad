use super::filter::{should_keep_child, should_keep_command};
use super::ProcessMaps;
use std::collections::{HashMap, HashSet};

pub(super) fn parse_process_snapshot(
    stdout: &str,
    root_filter: Option<&HashSet<String>>,
) -> ProcessMaps {
    let mut commands = HashMap::new();
    let mut child_pids: HashMap<String, Vec<String>> = HashMap::new();

    for line in stdout.lines() {
        let Some((pid, ppid, args)) = parse_process_line(line) else {
            continue;
        };

        if should_keep_command(root_filter, pid, ppid) {
            commands.insert(pid.to_string(), args.to_string());
        }
        if should_keep_child(root_filter, ppid) {
            child_pids
                .entry(ppid.to_string())
                .or_default()
                .push(pid.to_string());
        }
    }

    (commands, child_pids)
}

fn parse_process_line(line: &str) -> Option<(&str, &str, &str)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let (pid, rest) = split_once_whitespace(trimmed)?;
    let rest = rest.trim_start();
    let (ppid, args) = split_once_whitespace(rest)?;
    let args = args.trim_start();
    if args.is_empty() {
        return None;
    }
    Some((pid, ppid, args))
}

fn split_once_whitespace(value: &str) -> Option<(&str, &str)> {
    let split_at = value.find(char::is_whitespace)?;
    Some((&value[..split_at], &value[split_at..]))
}

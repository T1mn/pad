use std::collections::HashSet;

pub(super) fn should_keep_command(
    root_filter: Option<&HashSet<String>>,
    pid: &str,
    ppid: &str,
) -> bool {
    match root_filter {
        Some(roots) => roots.contains(pid) || roots.contains(ppid),
        None => true,
    }
}

pub(super) fn should_keep_child(root_filter: Option<&HashSet<String>>, ppid: &str) -> bool {
    match root_filter {
        Some(roots) => roots.contains(ppid),
        None => true,
    }
}

pub(super) fn normalize_root_pids(root_pids: &[String]) -> HashSet<String> {
    root_pids
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|pid| !pid.is_empty())
        .map(str::to_string)
        .collect()
}

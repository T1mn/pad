mod command {
    use super::filter::normalize_root_pids;
    use super::parse::parse_process_snapshot;
    use super::ProcessMaps;
    use std::process::Command;

    pub(in crate::scanner::process_snapshot) fn load_lightweight_process_snapshot(
        root_pids: &[String],
    ) -> Option<ProcessMaps> {
        load_ps_snapshot(root_pids, &["-axo", "pid=,ppid=,comm="])
    }

    pub(in crate::scanner::process_snapshot) fn load_process_snapshot(
        root_pids: &[String],
    ) -> Option<ProcessMaps> {
        load_ps_snapshot(root_pids, &["-axo", "pid=,ppid=,args="])
    }

    fn load_ps_snapshot(root_pids: &[String], args: &[&str]) -> Option<ProcessMaps> {
        let output = Command::new("ps").args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }

        let roots = normalize_root_pids(root_pids);
        Some(parse_process_snapshot(
            &String::from_utf8_lossy(&output.stdout),
            (!roots.is_empty()).then_some(&roots),
        ))
    }
}
mod filter {
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

    pub(in crate::scanner::process_snapshot) fn normalize_root_pids(
        root_pids: &[String],
    ) -> HashSet<String> {
        root_pids
            .iter()
            .map(String::as_str)
            .map(str::trim)
            .filter(|pid| !pid.is_empty())
            .map(str::to_string)
            .collect()
    }
}
mod parse;

pub(super) use command::{load_lightweight_process_snapshot, load_process_snapshot};
#[cfg(test)]
pub(in crate::scanner::process_snapshot) use filter::normalize_root_pids;
#[cfg(test)]
pub(in crate::scanner::process_snapshot) use parse::parse_process_snapshot;
use std::collections::HashMap;

pub(super) type ProcessMaps = (HashMap<String, String>, HashMap<String, Vec<String>>);

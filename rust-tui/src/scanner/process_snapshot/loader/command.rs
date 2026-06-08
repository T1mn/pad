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

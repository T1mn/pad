use super::process_snapshot::command_args_may_name_agent;
use super::scan_caches::ScanCaches;
use crate::model::AgentType;

pub(super) fn detect_agent_type(
    current_cmd: &str,
    pane_pid: &str,
    caches: &mut ScanCaches,
) -> (AgentType, String, String) {
    let current_process = current_cmd.trim().to_string();
    let mut agent_type = AgentType::from_processes(&current_process);
    if !matches!(agent_type, AgentType::Unknown) {
        return (agent_type, current_process, String::new());
    }

    let main_process = caches.cached_process_cmd(pane_pid).unwrap_or_default();
    agent_type = AgentType::from_processes(&main_process);
    if !matches!(agent_type, AgentType::Unknown) {
        return (agent_type, main_process, String::new());
    }

    if command_args_may_name_agent(&main_process) {
        let full_process = caches.cached_full_process_cmd(pane_pid).unwrap_or_default();
        agent_type = AgentType::from_processes(&full_process);
        if !matches!(agent_type, AgentType::Unknown) {
            return (agent_type, full_process, String::new());
        }
    }

    let child_processes = caches.cached_child_processes(pane_pid);
    agent_type = AgentType::from_processes(&child_processes);
    (agent_type, main_process, child_processes)
}

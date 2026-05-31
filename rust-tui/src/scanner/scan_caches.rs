use super::git::get_git_info;
use super::process_snapshot::ProcessSnapshot;
use crate::model::GitInfo;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct ScanCaches {
    pub(super) git_info: HashMap<String, Option<GitInfo>>,
    pub(super) processes: ProcessSnapshot,
}

impl ScanCaches {
    pub(super) fn with_pane_pids(pane_pids: Vec<String>) -> Self {
        Self {
            git_info: HashMap::new(),
            processes: ProcessSnapshot::for_root_pids(pane_pids),
        }
    }

    pub(super) fn cached_process_cmd(&mut self, pid: &str) -> Option<String> {
        self.processes.command(pid)
    }

    pub(super) fn cached_full_process_cmd(&mut self, pid: &str) -> Option<String> {
        self.processes.full_command(pid)
    }

    pub(super) fn cached_child_processes(&mut self, pid: &str) -> String {
        self.processes.child_processes(pid)
    }

    pub(super) fn cached_git_info(&mut self, working_dir: &str) -> Option<GitInfo> {
        if let Some(info) = self.git_info.get(working_dir) {
            return info.clone();
        }

        let info = get_git_info(working_dir);
        self.git_info.insert(working_dir.to_string(), info.clone());
        info
    }
}

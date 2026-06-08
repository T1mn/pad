use super::git::get_git_info_for_paths;
use super::pane_capture::{capture_pane_content, capture_panes_content};
use super::scan_caches::ScanCaches;
use crate::model::{AgentPanel, AgentState};

pub(super) fn hydrate_panel_runtime_state(panels: &mut [AgentPanel], caches: &mut ScanCaches) {
    let pane_ids = panels
        .iter()
        .map(|panel| panel.pane_id.clone())
        .collect::<Vec<_>>();
    let working_dirs = panels
        .iter()
        .map(|panel| panel.working_dir.clone())
        .collect::<Vec<_>>();
    let (captures, git_infos) = std::thread::scope(|scope| {
        let capture_handle =
            scope.spawn(|| capture_panes_content(&pane_ids, 20).unwrap_or_default());
        let git_handle = scope.spawn(|| get_git_info_for_paths(&working_dirs));
        (
            capture_handle.join().unwrap_or_default(),
            git_handle.join().unwrap_or_default(),
        )
    });

    for panel in panels {
        let state = captures
            .get(&panel.pane_id)
            .map(|content| crate::detector::detect_state(content))
            .or_else(|| {
                capture_pane_content(&panel.pane_id, 20)
                    .ok()
                    .map(|content| crate::detector::detect_state(&content))
            })
            .unwrap_or(AgentState::Idle);
        panel.is_active = state == AgentState::Busy;
        panel.state = state;
        panel.git_info = git_infos
            .get(&panel.working_dir)
            .cloned()
            .unwrap_or_else(|| caches.cached_git_info(&panel.working_dir));
    }
}

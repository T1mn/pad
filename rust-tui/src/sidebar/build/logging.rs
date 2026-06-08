use std::time::{Duration, Instant};

pub(super) fn log_sidebar_stage(
    label: &str,
    started_at: Instant,
    folder_count: usize,
    item_count: usize,
) {
    let elapsed = started_at.elapsed();
    if elapsed >= Duration::from_millis(8) {
        crate::log_debug!(
            "sidebar.build: stage={} elapsed_ms={} folders={} items={}",
            label,
            elapsed.as_millis(),
            folder_count,
            item_count
        );
    }
}

pub(super) fn log_slow_folder(path: &str, thread_count: usize, started_at: Instant) {
    if started_at.elapsed() >= Duration::from_millis(20) {
        crate::log_debug!(
            "sidebar.build: folder_slow path={} threads={} elapsed_ms={}",
            path,
            thread_count,
            started_at.elapsed().as_millis()
        );
    }
}

pub(super) struct BuildLogStats {
    pub(super) live_panel_threads: usize,
    pub(super) hidden_live_panels: usize,
    pub(super) codex_history_threads: usize,
    pub(super) claude_history_threads: usize,
    pub(super) gemini_history_threads: usize,
    pub(super) opencode_history_threads: usize,
}

impl BuildLogStats {
    pub(super) fn new() -> Self {
        Self {
            live_panel_threads: 0,
            hidden_live_panels: 0,
            codex_history_threads: 0,
            claude_history_threads: 0,
            gemini_history_threads: 0,
            opencode_history_threads: 0,
        }
    }
}

pub(super) fn log_total_build(started_at: Instant, folder_count: usize, stats: &BuildLogStats) {
    if started_at.elapsed() >= Duration::from_millis(20) {
        crate::log_debug!(
            "sidebar.build: total elapsed_ms={} folders={} live_threads={} hidden_live_panels={} codex_history_threads={} claude_history_threads={} gemini_history_threads={} opencode_history_threads={}",
            started_at.elapsed().as_millis(),
            folder_count,
            stats.live_panel_threads,
            stats.hidden_live_panels,
            stats.codex_history_threads,
            stats.claude_history_threads,
            stats.gemini_history_threads,
            stats.opencode_history_threads
        );
    }
}

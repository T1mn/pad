use super::super::tmux::current_tmux_client_snapshot;

pub(super) fn log_client_context(trace_id: &str) {
    log_debug!(
        "handoff trace={} stage=create.client_context snapshot={}",
        trace_id,
        current_tmux_client_snapshot().as_deref().unwrap_or("-")
    );
}

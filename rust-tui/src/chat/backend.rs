mod panels;
mod status {
    use crate::runtime_status;

    pub(crate) fn pad_is_online() -> bool {
        runtime_status::read_status(&crate::paths::pad_status_path())
            .map(|status| runtime_status::process_alive(status.pid))
            .unwrap_or(false)
    }
}
mod text;

pub(crate) use panels::{invalidate_live_panels, live_panels};
pub(crate) use status::pad_is_online;
pub(crate) use text::{
    build_slash_command_text, compact_target_label, panel_display_title, summarize_pane_capture,
};

#[cfg(test)]
use text::leaf_name;

#[cfg(test)]
mod tests;

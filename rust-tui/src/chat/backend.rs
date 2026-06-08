mod panels;
mod status;
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

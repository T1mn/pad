mod basic;
mod sound;

pub(super) use basic::{
    handle_auto_refresh_detail_mode, handle_claude_full_access_detail_mode,
    handle_display_mode_detail_mode, handle_preview_mode_detail_mode, handle_trash_detail_mode,
};
pub(super) use sound::handle_sound_detail_mode;

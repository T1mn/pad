use crate::opencode_text::{
    extract_display_part_text, message_role as parse_message_role, OpenCodeRole,
};
use crate::preview_source::turns::SessionRole;

pub(super) fn message_role(raw: &str) -> Option<SessionRole> {
    match parse_message_role(raw)? {
        OpenCodeRole::User => Some(SessionRole::User),
        OpenCodeRole::Assistant => Some(SessionRole::Assistant),
    }
}

pub(super) fn extract_part_text(raw: &str) -> Option<String> {
    extract_display_part_text(raw)
}

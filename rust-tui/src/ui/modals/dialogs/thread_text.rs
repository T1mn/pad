mod action;
mod meta;
mod subject {
    pub(in crate::ui::modals::dialogs) fn thread_action_subject(
        title: &str,
        session_id: Option<&str>,
    ) -> String {
        let title = title.trim();
        if !title.is_empty() && title != "untitled" {
            title.to_string()
        } else {
            session_id.unwrap_or("unknown session").to_string()
        }
    }
}

pub(super) use action::{
    thread_action_cancel_hint, thread_action_confirm_hint, thread_action_live_warning,
    thread_action_modal_confirm, thread_action_modal_title,
};
pub(super) use meta::{
    thread_meta_editor_field_label, thread_meta_editor_help_text, thread_meta_editor_prompt_text,
    thread_meta_editor_title,
};
pub(super) use subject::thread_action_subject;

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

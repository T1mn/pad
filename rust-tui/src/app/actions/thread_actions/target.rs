use crate::app::state::ThreadListView;
use crate::app::App;
use crate::model::AgentType;
use crate::sidebar::{SidebarItem, SidebarThread};

pub(super) fn selected_thread_action_target(
    app: &mut App,
    archived: bool,
) -> Option<SidebarThread> {
    match app.selected_sidebar_item()? {
        SidebarItem::Thread(thread)
            if matches!(
                thread.agent_type,
                AgentType::Codex | AgentType::Claude | AgentType::Gemini | AgentType::OpenCode
            ) && (app.thread_list_view() == ThreadListView::Trash
                || thread.archived == archived)
                && (app.thread_list_view() != ThreadListView::Trash || thread.deleted)
                && thread.session_id.is_some() =>
        {
            Some(thread.as_ref().clone())
        }
        _ => None,
    }
}

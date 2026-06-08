mod claude;
mod codex;
mod gemini;
mod opencode;

use crate::sidebar::model::SidebarThread;
use crate::thread_meta::ThreadMetaKey;

pub(super) fn thread_for_deleted_meta(key: &ThreadMetaKey) -> Option<SidebarThread> {
    match key.agent_type.as_str() {
        "codex" => codex::deleted_codex_thread(&key.thread_id),
        "claude" => claude::deleted_claude_thread(&key.thread_id),
        "gemini" => gemini::deleted_gemini_thread(&key.thread_id),
        "opencode" => opencode::deleted_opencode_thread(&key.thread_id),
        _ => None,
    }
}

use crate::app::actions::helpers::{
    failure_toast_title, success_toast_title, thread_action_subject,
};
use crate::app::{App, Mode, PendingThreadAction, ThreadActionKind};
use crate::model::AgentType;
use crate::sidebar::SidebarThread;

impl App {
    pub fn open_thread_action_confirm(&mut self, thread: SidebarThread, kind: ThreadActionKind) {
        self.sidebar.pending_thread_action = Some(PendingThreadAction { thread, kind });
        self.sidebar.thread_meta_editing = false;
        self.sidebar.thread_meta_target = None;
        self.sidebar.thread_meta_buffer.clear();
        self.mode = Mode::ThreadActionConfirm;
        self.dirty = true;
    }

    pub fn close_thread_action_confirm(&mut self) {
        self.sidebar.pending_thread_action = None;
        self.sidebar.thread_meta_editing = false;
        self.sidebar.thread_meta_target = None;
        self.sidebar.thread_meta_buffer.clear();
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    pub fn confirm_thread_action(&mut self) -> bool {
        let Some(action) = self.sidebar.pending_thread_action.clone() else {
            self.mode = Mode::Normal;
            self.dirty = true;
            return false;
        };
        self.sidebar.pending_thread_action = None;
        self.mode = Mode::Normal;

        let Some(session_id) = action.thread.session_id.as_deref() else {
            self.dirty = true;
            return false;
        };

        let result = execute_thread_action(&action, session_id);
        let ok = result.is_ok();

        match &result {
            Ok(()) => self.show_action_toast(
                success_toast_title(self.locale, action.kind, action.thread.agent_type.clone()),
                &thread_action_subject(&action.thread),
            ),
            Err(err) => self.show_action_toast(
                failure_toast_title(self.locale, action.kind, action.thread.agent_type.clone()),
                &err.to_string(),
            ),
        }

        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        self.invalidate_preview();
        self.focus_panel();
        self.dirty = true;
        ok
    }
}

fn execute_thread_action(action: &PendingThreadAction, session_id: &str) -> std::io::Result<()> {
    match action.kind {
        ThreadActionKind::Archive => archive_thread(&action.thread.agent_type, session_id),
        ThreadActionKind::Unarchive => unarchive_thread(&action.thread.agent_type, session_id),
        ThreadActionKind::Restore => crate::thread_meta::set_thread_deleted(
            &action.thread.agent_type.to_string(),
            session_id,
            false,
        ),
    }
}

fn archive_thread(agent_type: &AgentType, session_id: &str) -> std::io::Result<()> {
    match agent_type {
        AgentType::Codex => crate::codex_state::archive_thread(session_id),
        AgentType::Claude => crate::claude_history::archive_thread(session_id),
        AgentType::Gemini => crate::gemini_history::archive_thread(session_id),
        AgentType::OpenCode => crate::opencode_history::archive_thread(session_id),
        _ => unsupported_thread_action("archive"),
    }
}

fn unarchive_thread(agent_type: &AgentType, session_id: &str) -> std::io::Result<()> {
    match agent_type {
        AgentType::Codex => crate::codex_state::unarchive_thread(session_id),
        AgentType::Claude => crate::claude_history::unarchive_thread(session_id),
        AgentType::Gemini => crate::gemini_history::unarchive_thread(session_id),
        AgentType::OpenCode => crate::opencode_history::unarchive_thread(session_id),
        _ => unsupported_thread_action("restore"),
    }
}

fn unsupported_thread_action(action: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("{action} is not supported for this agent type"),
    ))
}

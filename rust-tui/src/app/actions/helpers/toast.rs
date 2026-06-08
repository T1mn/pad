use super::*;

pub(in crate::app::actions) fn thread_action_subject(thread: &SidebarThread) -> String {
    if !thread.title.trim().is_empty() && thread.title != "untitled" {
        thread.title.clone()
    } else {
        thread
            .session_id
            .clone()
            .unwrap_or_else(|| thread.key.clone())
    }
}

pub(in crate::app::actions) fn success_toast_title(
    locale: Locale,
    kind: ThreadActionKind,
    agent_type: AgentType,
) -> &'static str {
    match (is_cjk_locale(locale), kind, agent_type) {
        (true, ThreadActionKind::Archive, AgentType::Gemini) => "已在 pad 侧归档",
        (true, ThreadActionKind::Unarchive, AgentType::Gemini) => "已从 pad 侧恢复",
        (true, ThreadActionKind::Restore, AgentType::Gemini) => "已从回收站恢复",
        (false, ThreadActionKind::Archive, AgentType::Gemini) => "Pad archived",
        (false, ThreadActionKind::Unarchive, AgentType::Gemini) => "Pad restored",
        (false, ThreadActionKind::Restore, AgentType::Gemini) => "Restored from Trash",
        (true, ThreadActionKind::Archive, _) => "已归档",
        (true, ThreadActionKind::Unarchive, _) => "已恢复",
        (true, ThreadActionKind::Restore, _) => "已恢复",
        (false, ThreadActionKind::Archive, _) => "Archived",
        (false, ThreadActionKind::Unarchive, _) => "Restored",
        (false, ThreadActionKind::Restore, _) => "Restored",
    }
}

pub(in crate::app::actions) fn failure_toast_title(
    locale: Locale,
    kind: ThreadActionKind,
    agent_type: AgentType,
) -> &'static str {
    match (is_cjk_locale(locale), kind, agent_type) {
        (true, ThreadActionKind::Archive, AgentType::Gemini) => "Pad 归档失败",
        (true, ThreadActionKind::Unarchive, AgentType::Gemini) => "Pad 恢复失败",
        (true, ThreadActionKind::Restore, AgentType::Gemini) => "回收站恢复失败",
        (false, ThreadActionKind::Archive, AgentType::Gemini) => "Pad archive failed",
        (false, ThreadActionKind::Unarchive, AgentType::Gemini) => "Pad restore failed",
        (false, ThreadActionKind::Restore, AgentType::Gemini) => "Trash restore failed",
        (true, ThreadActionKind::Archive, _) => "归档失败",
        (true, ThreadActionKind::Unarchive, _) => "恢复失败",
        (true, ThreadActionKind::Restore, _) => "恢复失败",
        (false, ThreadActionKind::Archive, _) => "Archive Failed",
        (false, ThreadActionKind::Unarchive, _) => "Restore Failed",
        (false, ThreadActionKind::Restore, _) => "Restore Failed",
    }
}

pub(in crate::app::actions) fn delete_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "删除失败"
    } else {
        "Delete Failed"
    }
}

pub(in crate::app::actions) fn delete_hide_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "已删除，但隐藏失败"
    } else {
        "Deleted, But Hide Failed"
    }
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

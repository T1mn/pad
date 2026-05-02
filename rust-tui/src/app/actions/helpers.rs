use super::*;

pub(crate) fn settings_item_search_blob(
    locale: Locale,
    id: &str,
    value: &str,
    name_key: &str,
    desc_key: &str,
) -> String {
    let mut terms = vec![
        id.to_lowercase(),
        id.replace('_', " ").to_lowercase(),
        name_key.to_lowercase(),
        name_key.replace(['.', '_'], " ").to_lowercase(),
        desc_key.to_lowercase(),
        desc_key.replace(['.', '_'], " ").to_lowercase(),
        value.to_lowercase(),
        crate::i18n::t(locale, name_key).to_lowercase(),
        crate::i18n::t(locale, desc_key).to_lowercase(),
        crate::i18n::t(Locale::En, name_key).to_lowercase(),
        crate::i18n::t(Locale::En, desc_key).to_lowercase(),
    ];
    terms.extend(
        settings_item_aliases(id)
            .iter()
            .map(|alias| alias.to_string()),
    );
    terms.join(" ")
}

fn settings_item_aliases(id: &str) -> &'static [&'static str] {
    match id {
        "theme" => &["theme", "color theme", "appearance"],
        "auto_refresh" => &["auto refresh", "refresh", "refresh interval"],
        "codex_settings" => &[
            "codex",
            "codex settings",
            "codex full access",
            "codex permissions",
            "approval policy",
            "sandbox mode",
            "yolo",
            "fast",
            "goal",
            "goals",
            "/goal",
            "service tier",
            "multi agent",
            "subagents",
            "web search",
            "search mode",
        ],
        "claude_full_access" => &[
            "claude",
            "claude full access",
            "claude permissions",
            "bypass permissions",
            "sandbox",
        ],
        "sound" => &[
            "sound",
            "audio",
            "notification sound",
            "completion sound",
            "approval sound",
            "timeout sound",
            "failure sound",
            "beep",
            "chime",
        ],
        "relay" => &["relay", "provider", "model provider", "proxy"],
        "telegram" => &["telegram", "bot", "telegram bot"],
        "agent_style" => &["agent style", "attach style", "status bar", "zoom"],
        "preview_mode" => &[
            "preview",
            "preview mode",
            "preview source",
            "session preview",
        ],
        "display_mode" => &[
            "display",
            "display mode",
            "display settings",
            "session scope",
        ],
        "trash" => &["trash", "recycle bin", "deleted", "deleted threads"],
        "language" => &["language", "locale"],
        "version" => &["version", "about"],
        _ => &[],
    }
}

pub(super) fn thread_action_subject(thread: &SidebarThread) -> String {
    if !thread.title.trim().is_empty() && thread.title != "untitled" {
        thread.title.clone()
    } else {
        thread
            .session_id
            .clone()
            .unwrap_or_else(|| thread.key.clone())
    }
}

pub(super) fn success_toast_title(
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

pub(super) fn parse_thread_tags(input: &str) -> Vec<String> {
    input
        .split([',', '\n', ';'])
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_string())
        .collect()
}

pub(super) fn thread_meta_save_failed_title(locale: Locale) -> &'static str {
    if matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja) {
        "保存失败"
    } else {
        "Save failed"
    }
}

pub(super) fn thread_meta_toast(
    locale: Locale,
    kind: ThreadMetaEditKind,
    input: &str,
) -> (&'static str, String) {
    let empty_title = matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja);
    match kind {
        ThreadMetaEditKind::Title => {
            if input.is_empty() {
                if empty_title {
                    ("标题已清空", String::from("将回退到上游标题"))
                } else {
                    (
                        "Title cleared",
                        String::from("Will fall back to upstream title"),
                    )
                }
            } else if empty_title {
                ("标题已保存", input.to_string())
            } else {
                ("Title saved", input.to_string())
            }
        }
        ThreadMetaEditKind::Tags => {
            if input.is_empty() {
                if empty_title {
                    ("标签已清空", String::from("无标签"))
                } else {
                    ("Tags cleared", String::from("No tags"))
                }
            } else if empty_title {
                ("标签已保存", input.to_string())
            } else {
                ("Tags saved", input.to_string())
            }
        }
    }
}

pub(super) fn failure_toast_title(
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

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

pub(super) fn delete_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "删除失败"
    } else {
        "Delete Failed"
    }
}

pub(super) fn delete_hide_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "已删除，但隐藏失败"
    } else {
        "Deleted, But Hide Failed"
    }
}

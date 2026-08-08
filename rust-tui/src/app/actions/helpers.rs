use super::*;

mod locale {
    use crate::i18n::Locale;

    pub(in crate::app::actions) fn is_cjk_locale(locale: Locale) -> bool {
        matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
    }

    pub(in crate::app::actions) fn localized(
        locale: Locale,
        zh: &'static str,
        en: &'static str,
    ) -> &'static str {
        if is_cjk_locale(locale) {
            zh
        } else {
            en
        }
    }
}
mod quote {
    pub(in crate::app::actions) fn trim_wrapping_quotes(value: &str) -> &str {
        value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .or_else(|| {
                value
                    .strip_prefix('\'')
                    .and_then(|value| value.strip_suffix('\''))
            })
            .unwrap_or(value)
    }
}
mod settings_search;
mod thread_meta {
    use super::*;

    pub(in crate::app::actions) fn parse_thread_tags(input: &str) -> Vec<String> {
        input
            .split([',', '\n', ';'])
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
            .map(|tag| tag.to_string())
            .collect()
    }

    pub(in crate::app::actions) fn thread_meta_save_failed_title(locale: Locale) -> &'static str {
        localized(locale, "保存失败", "Save failed")
    }

    pub(in crate::app::actions) fn thread_meta_toast(
        locale: Locale,
        kind: ThreadMetaEditKind,
        input: &str,
    ) -> (&'static str, String) {
        let empty_title = is_cjk_locale(locale);
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
}
mod toast {
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
        localized(locale, "删除失败", "Delete Failed")
    }

    pub(in crate::app::actions) fn delete_hide_failed_title(locale: Locale) -> &'static str {
        localized(locale, "已删除，但隐藏失败", "Deleted, But Hide Failed")
    }
}

pub(in crate::app::actions) use locale::{is_cjk_locale, localized};
pub(in crate::app::actions) use quote::trim_wrapping_quotes;
pub(crate) use settings_search::{settings_item_matches_search, settings_item_search_blob};
pub(super) use thread_meta::{parse_thread_tags, thread_meta_save_failed_title, thread_meta_toast};
pub(super) use toast::{
    delete_failed_title, delete_hide_failed_title, failure_toast_title, success_toast_title,
    thread_action_subject,
};

use super::mode::ExportMode;
use crate::i18n::Locale;

pub(super) fn export_saved_title(locale: Locale, mode: ExportMode) -> &'static str {
    match (is_cjk_locale(locale), mode) {
        (true, ExportMode::Raw) => "OpenCode 已导出",
        (true, ExportMode::Sanitized) => "OpenCode 已脱敏导出",
        (false, ExportMode::Raw) => "OpenCode Exported",
        (false, ExportMode::Sanitized) => "OpenCode Sanitized Exported",
    }
}

pub(super) fn export_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode 导出失败", "OpenCode Export Failed")
}

pub(super) fn no_thread_message(locale: Locale) -> &'static str {
    localized(locale, "没有选中的线程", "No selected thread")
}

pub(super) fn opencode_only_message(locale: Locale) -> &'static str {
    localized(locale, "只支持 OpenCode 会话", "Only OpenCode sessions")
}

pub(super) fn missing_session_message(locale: Locale) -> &'static str {
    localized(
        locale,
        "选中的 OpenCode 线程缺少 session id",
        "Missing OpenCode session id",
    )
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

fn localized(locale: Locale, zh: &'static str, en: &'static str) -> &'static str {
    if is_cjk_locale(locale) {
        zh
    } else {
        en
    }
}

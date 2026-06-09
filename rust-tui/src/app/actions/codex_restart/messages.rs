use super::super::helpers::{is_cjk_locale, localized};
use crate::i18n::Locale;

pub(super) fn restart_started_title(locale: Locale) -> &'static str {
    localized(locale, "Codex 重启中", "Codex Restarting")
}

pub(super) fn restart_failed_title(locale: Locale) -> &'static str {
    localized(locale, "Codex 重启失败", "Codex Restart Failed")
}

pub(super) fn restart_started_body(locale: Locale, session_id: Option<&str>) -> String {
    let session = session_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or("--last");
    if is_cjk_locale(locale) {
        format!("恢复会话 {session}")
    } else {
        format!("Resuming {session}")
    }
}

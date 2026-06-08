use crate::app::{App, CodexCliVersionInfo};

pub(super) fn show_codex_update_success_toast(app: &mut App, info: &CodexCliVersionInfo) {
    let local = info
        .local_version
        .clone()
        .unwrap_or_else(|| "?".to_string());
    let latest = info
        .latest_version
        .clone()
        .unwrap_or_else(|| "?".to_string());
    if is_chinese_locale(app) {
        app.show_action_toast("Codex 升级完成", &format!("当前 {local} · 最新 {latest}"));
    } else {
        app.show_action_toast(
            "Codex updated",
            &format!("Current {local} · latest {latest}"),
        );
    }
}

pub(super) fn show_codex_update_failure_toast(app: &mut App, err: &str) {
    if is_chinese_locale(app) {
        app.show_action_toast("Codex 升级失败", err);
    } else {
        app.show_action_toast("Codex update failed", err);
    }
}

fn is_chinese_locale(app: &App) -> bool {
    matches!(
        app.locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    )
}

pub(super) fn telegram_label(locale: crate::i18n::Locale, key: &str) -> String {
    let zh = locale_prefers_chinese(locale);
    match key {
        "enabled" if zh => "启用".to_string(),
        "enabled" => "Enabled".to_string(),
        "bot_token" if zh => "Bot Token".to_string(),
        "bot_token" => "Bot Token".to_string(),
        "chat_id" if zh => "Chat ID".to_string(),
        "chat_id" => "Chat ID".to_string(),
        "restart_bot" if zh => "重启 Bot".to_string(),
        "restart_bot" => "Restart Bot".to_string(),
        "bot_username" if zh => "Bot Username".to_string(),
        "bot_username" => "Bot Username".to_string(),
        "pad_status" if zh => "Pad 状态".to_string(),
        "pad_status" => "Pad Status".to_string(),
        "bot_status" if zh => "Bot 守护进程".to_string(),
        "bot_status" => "Bot Daemon".to_string(),
        _ => key.to_string(),
    }
}

pub(super) fn restart_value(locale: crate::i18n::Locale) -> String {
    if locale_prefers_chinese(locale) {
        "立即重启".to_string()
    } else {
        "Restart now".to_string()
    }
}

pub(super) fn footer_text(locale: crate::i18n::Locale, editing: bool) -> &'static str {
    match (locale_prefers_chinese(locale), editing) {
        (true, true) => "输入编辑 | Enter: 保存 | Shift+Delete: 清空 | Esc: 取消",
        (false, true) => "Type to edit | Enter: save | Shift+Delete: clear | Esc: cancel",
        (true, false) => "j/k: 移动 | Enter/Space: 编辑/切换/重启 | r: 重启 | Esc: 返回",
        (false, false) => "j/k: move | Enter/Space: edit/toggle/restart | r: restart | Esc: back",
    }
}

fn locale_prefers_chinese(locale: crate::i18n::Locale) -> bool {
    matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    )
}

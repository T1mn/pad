use super::common::render_modal_surface;
use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn draw_telegram_settings_modal(f: &mut Frame, app: &App) {
    use crate::runtime_status;

    let theme = &app.theme;
    let locale = app.locale;
    let area = crate::ui::layout::popup_area(72, 13, f.area());
    render_modal_surface(f, area, theme);

    let block = Block::default()
        .title(" ✈ Telegram ".to_string())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(3),
    };

    let enabled_value = if app.config.telegram.enabled {
        crate::i18n::t(locale, "settings.on").to_string()
    } else {
        crate::i18n::t(locale, "settings.off").to_string()
    };
    let token_value = if app.telegram_editing && app.telegram_selected_field == 1 {
        format!("{}|", app.telegram_edit_buffer)
    } else {
        mask_secret(&app.config.telegram.bot_token)
    };
    let chat_value = if app.telegram_editing && app.telegram_selected_field == 2 {
        format!("{}|", app.telegram_edit_buffer)
    } else if app.config.telegram.chat_id.is_empty() {
        "(empty)".to_string()
    } else {
        app.config.telegram.chat_id.clone()
    };
    let username_value = if app.config.telegram.bot_username.is_empty() {
        "(unknown)".to_string()
    } else {
        format!("@{}", app.config.telegram.bot_username)
    };
    let pad_status = runtime_status::describe_status(&crate::paths::pad_status_path());
    let bot_status = runtime_status::describe_status(&crate::paths::telegram_bot_status_path());
    let restart_value = if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    ) {
        "立即重启".to_string()
    } else {
        "Restart now".to_string()
    };

    let rows = vec![
        telegram_row(
            app,
            0,
            &telegram_label(locale, "enabled"),
            &enabled_value,
            true,
        ),
        telegram_row(
            app,
            1,
            &telegram_label(locale, "bot_token"),
            &token_value,
            true,
        ),
        telegram_row(
            app,
            2,
            &telegram_label(locale, "chat_id"),
            &chat_value,
            true,
        ),
        telegram_row(
            app,
            3,
            &telegram_label(locale, "restart_bot"),
            &restart_value,
            true,
        ),
        telegram_row(
            app,
            99,
            &telegram_label(locale, "bot_username"),
            &username_value,
            false,
        ),
        telegram_row(
            app,
            99,
            &telegram_label(locale, "pad_status"),
            &pad_status,
            false,
        ),
        telegram_row(
            app,
            99,
            &telegram_label(locale, "bot_status"),
            &bot_status,
            false,
        ),
    ];

    let table = Table::new(rows, [Constraint::Length(18), Constraint::Min(0)]);
    f.render_widget(table, inner);

    let footer_text = if app.telegram_editing {
        if matches!(
            locale,
            crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
        ) {
            "输入编辑 | Enter: 保存 | Esc: 取消"
        } else {
            "Type to edit | Enter: save | Esc: cancel"
        }
    } else if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    ) {
        "j/k: 移动 | Enter/Space: 编辑/切换/重启 | r: 重启 | Esc: 返回"
    } else {
        "j/k: move | Enter/Space: edit/toggle/restart | r: restart | Esc: back"
    };
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.comment));
    let footer_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 1,
    };
    f.render_widget(footer, footer_area);
}

fn telegram_row(
    app: &App,
    field_idx: usize,
    name: &str,
    value: &str,
    editable: bool,
) -> Row<'static> {
    let theme = &app.theme;
    let is_selected = editable && field_idx == app.telegram_selected_field;
    let name_style = if is_selected {
        Style::default()
            .bg(theme.highlight_bg)
            .fg(theme.highlight_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg)
    };
    let value_style = if is_selected {
        Style::default().bg(theme.highlight_bg).fg(theme.accent)
    } else if editable {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.comment)
    };
    Row::new(vec![
        Cell::from(name.to_string()).style(name_style),
        Cell::from(value.to_string()).style(value_style),
    ])
}

fn telegram_label(locale: crate::i18n::Locale, key: &str) -> String {
    let zh = matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    );
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

fn mask_secret(secret: &str) -> String {
    if secret.is_empty() {
        return "(empty)".to_string();
    }
    let chars = secret.chars().collect::<Vec<_>>();
    if chars.len() <= 10 {
        return "*".repeat(chars.len());
    }
    let head = chars.iter().take(4).collect::<String>();
    let tail = chars
        .iter()
        .rev()
        .take(4)
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{}…{}", head, tail)
}

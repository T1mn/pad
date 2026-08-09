mod labels {
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
            (false, false) => {
                "j/k: move | Enter/Space: edit/toggle/restart | r: restart | Esc: back"
            }
        }
    }

    fn locale_prefers_chinese(locale: crate::i18n::Locale) -> bool {
        matches!(
            locale,
            crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
        )
    }
}
mod row {
    use crate::app::App;
    use ratatui::{
        style::{Modifier, Style},
        widgets::{Cell, Row},
    };

    use super::values::TelegramRowValue;

    pub(super) fn telegram_row(app: &App, row: TelegramRowValue) -> Row<'static> {
        let theme = &app.theme;
        let is_selected = row.editable && row.field_idx == app.telegram_selected_field;
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
        } else if row.editable {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.comment)
        };
        Row::new(vec![
            Cell::from(row.name).style(name_style),
            Cell::from(row.value).style(value_style),
        ])
    }
}
pub(crate) mod values;

use super::common::render_modal_surface;
use crate::app::App;
use labels::footer_text;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph, Table},
    Frame,
};
use row::telegram_row;
use values::telegram_rows;

pub fn draw_telegram_settings_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = crate::ui::layout::popup_area(72, 13, f.area());
    render_modal_surface(f, area, theme);
    draw_telegram_settings_content(f, app, area, true);
}

pub(super) fn draw_telegram_settings_content(
    f: &mut Frame,
    app: &App,
    area: Rect,
    show_footer: bool,
) {
    let theme = &app.theme;
    let inner = if show_footer {
        let block = Block::default()
            .title(" ✈ Telegram ".to_string())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.accent));
        f.render_widget(block, area);
        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        }
    } else {
        area
    };

    let rows = telegram_rows(app)
        .into_iter()
        .map(|row| telegram_row(app, row))
        .collect::<Vec<_>>();
    let table = Table::new(rows, [Constraint::Length(18), Constraint::Min(0)]);
    f.render_widget(table, inner);

    if show_footer {
        let footer = Paragraph::new(footer_text(app.locale, app.telegram_editing))
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
}

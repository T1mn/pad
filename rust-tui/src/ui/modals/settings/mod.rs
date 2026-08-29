mod codex;
mod detail;
mod detail_lists;
mod launcher {
    use super::super::common::render_modal_surface;
    use crate::app::App;
    use crate::tree::AgentLauncher;
    use ratatui::{
        layout::{Constraint, Rect},
        style::{Modifier, Style},
        widgets::{Block, BorderType, Borders, Cell, Row, Table},
        Frame,
    };

    pub(super) fn draw_agent_launcher(
        f: &mut Frame,
        app: &App,
        launcher: &AgentLauncher,
        area: Rect,
    ) {
        let theme = &app.theme;
        let locale = app.locale;
        let popup_width = 50;
        let popup_height = 12;
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(
            area.x + popup_x,
            area.y + popup_y,
            popup_width,
            popup_height,
        );

        render_modal_surface(f, popup_area, theme);

        let items: Vec<Row> = launcher
            .agents
            .iter()
            .enumerate()
            .map(|(idx, (name, _))| {
                let prefix = if idx == launcher.selected {
                    "❯ "
                } else {
                    "  "
                };
                let cells = vec![Cell::from(format!("{}{}", prefix, name))];
                let style = if idx == launcher.selected {
                    Style::default()
                        .bg(theme.highlight_bg)
                        .fg(theme.highlight_fg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };
                Row::new(cells).style(style)
            })
            .collect();

        let title = format!(
            " {} {} ",
            crate::i18n::t(locale, "agent_launcher.title"),
            launcher.target_dir.display()
        );
        let block = Block::default()
            .title(title)
            .title_alignment(ratatui::layout::Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.accent));

        let table = Table::new(items, [Constraint::Percentage(100)]).block(block);

        f.render_widget(table, popup_area);
    }
}
mod layout;
mod list {
    use crate::app::App;
    use crate::i18n::Locale;
    use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
    use ratatui::{layout::Rect, Frame};

    pub(super) fn draw_settings_list(f: &mut Frame, app: &App, area: Rect) {
        let locale = app.locale;
        let items = app.filtered_settings_items();
        let selection_items: Vec<SelectionItem> = items
            .iter()
            .map(|(id, value, name_key, desc_key, editable)| SelectionItem {
                title: crate::i18n::t(locale, name_key).to_string(),
                value: Some(if *editable {
                    format!("{}  ›", value)
                } else {
                    value.clone()
                }),
                subtitle: Some(settings_list_description(locale, id, desc_key)),
                keyword: Some(crate::app::actions::settings_item_search_blob(
                    locale, id, value, name_key, desc_key,
                )),
                disabled: false,
            })
            .collect();
        let mut state = SelectionState {
            selected: app.settings_selected,
            scroll: 0,
            query: app.settings_search.clone(),
            searching: app.settings_searching,
        };
        state.clamp_selected(selection_items.len());
        render_selection_surface(
            f,
            area,
            &app.theme,
            crate::i18n::t(locale, "settings.title"),
            &selection_items,
            &state,
            Some(settings_list_footer(app.locale)),
        );
    }

    pub(super) fn settings_list_footer(locale: Locale) -> &'static str {
        match locale {
            Locale::ZhCN => "↑/↓ 或 j/k 移动 · Enter 打开 · / 搜索 · Esc 关闭",
            Locale::ZhTW => "↑/↓ 或 j/k 移動 · Enter 打開 · / 搜尋 · Esc 關閉",
            _ => "↑/↓ or j/k move · Enter open · / search · Esc close",
        }
    }

    fn settings_list_description(locale: Locale, id: &str, desc_key: &str) -> String {
        match locale {
            Locale::ZhCN => match id {
                "theme" => "切换整体配色方案",
                "auto_refresh" => "自动刷新 agent 和 session 列表",
                "codex_settings" => "配置 Codex 权限、速度、联网、状态栏和摘要",
                "profile" => "配置 PAD Profile 的默认权限模式和无人值守执行",
                "profile_permission_mode" => "切换 Profile 的默认权限模式",
                "profile_full_access" => "切换 Profile 的完全访问默认值",
                "profile_unattended" => "允许 Profile 在后台无人值守执行",
                "claude_full_access" => "启动时自动应用 Claude 高权限配置",
                "sound" => crate::i18n::t(locale, desc_key),
                "relay" => "配置各 agent 的 provider / proxy",
                "telegram" => "通过 Telegram 远程查看和控制 PAD",
                "preview_mode" => "选择右侧预览的数据来源",
                "display_mode" => "切换仅 live session 或全部 session",
                "trash" => "查看或恢复隐藏的线程",
                "language" => "切换界面语言",
                "version" => "当前 PAD 版本",
                _ => crate::i18n::t(locale, desc_key),
            },
            Locale::ZhTW => match id {
                "theme" => "切換整體配色方案",
                "auto_refresh" => "自動刷新 agent 和 session 列表",
                "codex_settings" => "配置 Codex 權限、速度、聯網、狀態列和摘要",
                "profile" => "配置 PAD Profile 的預設權限模式和無人值守執行",
                "profile_permission_mode" => "切換 Profile 的預設權限模式",
                "profile_full_access" => "切換 Profile 的完全存取預設值",
                "profile_unattended" => "允許 Profile 在背景無人值守執行",
                "claude_full_access" => "啟動時自動套用 Claude 高權限配置",
                "sound" => crate::i18n::t(locale, desc_key),
                "relay" => "配置各 agent 的 provider / proxy",
                "telegram" => "透過 Telegram 遠端查看和控制 PAD",
                "preview_mode" => "選擇右側預覽的資料來源",
                "display_mode" => "切換僅 live session 或全部 session",
                "trash" => "查看或復原隱藏的執行緒",
                "language" => "切換介面語言",
                "version" => "目前 PAD 版本",
                _ => crate::i18n::t(locale, desc_key),
            },
            _ => match id {
                "theme" => "Choose the color palette",
                "auto_refresh" => "Refresh agent and session lists automatically",
                "codex_settings" => {
                    "Configure Codex permissions, speed, web search, status line, and summaries"
                }
                "profile" => "Configure PAD profile permission and unattended defaults",
                "profile_permission_mode" => "Choose the profile default permission mode",
                "profile_full_access" => "Toggle profile Full Access by default",
                "profile_unattended" => "Allow profile tasks to run unattended",
                "claude_full_access" => "Apply Claude high-access launch settings automatically",
                "sound" => crate::i18n::t(locale, desc_key),
                "relay" => "Configure provider and proxy settings for agents",
                "telegram" => "View and control PAD remotely from Telegram",
                "preview_mode" => "Choose the data source for the right preview",
                "display_mode" => "Switch between live-only and all sessions",
                "trash" => "View or restore hidden threads",
                "language" => "Change the UI language",
                "version" => "Current PAD version",
                _ => crate::i18n::t(locale, desc_key),
            },
        }
        .to_string()
    }
}

use super::common::render_modal_surface;
use crate::app::App;
use crate::ui::layout::popup_area;
use ratatui::{layout::Rect, Frame};

pub fn draw_settings_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let (content_w, content_h) = layout::settings_modal_size(app);
    let area = popup_area(content_w, content_h, f.area());
    render_modal_surface(f, area, theme);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    if app.settings_focus == crate::app::state::SettingsFocus::Detail && !app.settings_searching {
        detail::draw_settings_detail_panel(f, app, inner);
    } else {
        list::draw_settings_list(f, app, inner);
    }
}

pub fn draw_agent_launcher(
    f: &mut Frame,
    app: &App,
    launcher: &crate::tree::AgentLauncher,
    area: Rect,
) {
    launcher::draw_agent_launcher(f, app, launcher, area);
}

#[cfg(test)]
#[path = "mod_tests.rs"]
pub(crate) mod tests;

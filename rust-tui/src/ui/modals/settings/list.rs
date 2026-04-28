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
            detail: None,
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
            "claude_full_access" => "启动时自动应用 Claude 高权限配置",
            "sound" => crate::i18n::t(locale, desc_key),
            "relay" => "配置各 agent 的 provider / proxy",
            "telegram" => "通过 Telegram 远程查看和控制 PAD",
            "agent_style" => "控制 agent 启动后的 tmux 缩放和状态栏",
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
            "claude_full_access" => "啟動時自動套用 Claude 高權限配置",
            "sound" => crate::i18n::t(locale, desc_key),
            "relay" => "配置各 agent 的 provider / proxy",
            "telegram" => "透過 Telegram 遠端查看和控制 PAD",
            "agent_style" => "控制 agent 啟動後的 tmux 縮放和狀態列",
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
            "claude_full_access" => "Apply Claude high-access launch settings automatically",
            "sound" => crate::i18n::t(locale, desc_key),
            "relay" => "Configure provider and proxy settings for agents",
            "telegram" => "View and control PAD remotely from Telegram",
            "agent_style" => "Control tmux zoom and status bar style after launch",
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

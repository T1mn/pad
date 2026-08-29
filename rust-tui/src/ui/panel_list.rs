mod animation {
    use crate::model::AgentState;
    #[cfg(test)]
    use ratatui::style::Modifier;
    use ratatui::style::{Color, Style};
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};

    #[cfg(test)]
    const SHIMMER_SWEEP_SECONDS: f32 = 2.0;
    #[cfg(test)]
    const SHIMMER_PADDING: f32 = 10.0;
    #[cfg(test)]
    const SHIMMER_BAND_HALF_WIDTH: f32 = 5.0;
    const BADGE_PULSE_CYCLE_SECONDS: f32 = 1.8;
    const BREATHING_MIN_VISIBLE_BLEND: f32 = 0.12;
    const BREATHING_BLEND_RANGE: f32 = 0.82;

    pub(super) fn thread_badge_breathes(state: &AgentState) -> bool {
        matches!(state, AgentState::Busy)
    }

    pub(super) fn breathing_badge_style(base_color: Color, surface_bg: Color, bg: Color) -> Style {
        let intensity = breathing_intensity();
        Style::default()
            .fg(super::style::blend_color(
                base_color,
                surface_bg,
                BREATHING_MIN_VISIBLE_BLEND + intensity * BREATHING_BLEND_RANGE,
            ))
            .bg(bg)
    }

    pub(super) fn breathing_badge_text() -> &'static str {
        "• "
    }

    #[cfg(test)]
    pub(super) fn shimmer_spans(
        text: &str,
        base_color: Color,
        highlight_color: Color,
        bg: Color,
    ) -> Vec<ratatui::text::Span<'static>> {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return Vec::new();
        }

        let total_width = super::metrics::display_width(text) as f32;
        let period = total_width + SHIMMER_PADDING * 2.0;
        let pos = (elapsed_since_start().as_secs_f32() % SHIMMER_SWEEP_SECONDS)
            / SHIMMER_SWEEP_SECONDS
            * period;
        let true_color = has_true_color();

        let mut spans = Vec::with_capacity(chars.len());
        let mut column = 0.0f32;

        for ch in chars {
            let width = super::metrics::char_display_width(ch).max(1) as f32;
            let center = column + (width * 0.5);
            let dist = ((center + SHIMMER_PADDING) - pos).abs();
            let intensity = if dist <= SHIMMER_BAND_HALF_WIDTH {
                let x = std::f32::consts::PI * (dist / SHIMMER_BAND_HALF_WIDTH);
                0.5 * (1.0 + x.cos())
            } else {
                0.0
            };
            let style = if true_color {
                let mixed = super::style::blend_color(highlight_color, base_color, intensity * 0.9);
                Style::default()
                    .fg(mixed)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                fallback_style(base_color, bg, intensity)
            };
            spans.push(ratatui::text::Span::styled(ch.to_string(), style));
            column += width;
        }

        spans
    }

    fn breathing_intensity() -> f32 {
        let phase = (elapsed_since_start().as_secs_f32() % BADGE_PULSE_CYCLE_SECONDS)
            / BADGE_PULSE_CYCLE_SECONDS;
        0.5 * (1.0 - (phase * std::f32::consts::TAU).cos())
    }

    fn elapsed_since_start() -> Duration {
        static PROCESS_START: OnceLock<Instant> = OnceLock::new();
        PROCESS_START.get_or_init(Instant::now).elapsed()
    }

    #[cfg(test)]
    fn has_true_color() -> bool {
        static HAS_TRUE_COLOR: OnceLock<bool> = OnceLock::new();
        *HAS_TRUE_COLOR.get_or_init(|| {
            let color_term = std::env::var("COLORTERM")
                .unwrap_or_default()
                .to_lowercase();
            if color_term.contains("truecolor") || color_term.contains("24bit") {
                return true;
            }

            let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
            term.contains("direct") || term.contains("truecolor") || term.contains("kitty")
        })
    }

    #[cfg(test)]
    fn fallback_style(base_color: Color, bg: Color, _intensity: f32) -> Style {
        Style::default().fg(base_color).bg(bg)
    }
}
mod draw;
mod empty {
    use super::labels::{
        special_view_empty_back_hint, special_view_empty_hint, special_view_empty_title,
    };
    use crate::app::state::ThreadListView;
    use crate::i18n::Locale;
    use crate::theme::Theme;
    use ratatui::{
        style::{Modifier, Style},
        text::{Line, Span},
    };

    pub(super) fn empty_message(
        locale: Locale,
        view: ThreadListView,
        theme: &Theme,
    ) -> Vec<Line<'static>> {
        if view != ThreadListView::Normal {
            return vec![
                Line::from(""),
                Line::from(Span::styled(
                    special_view_empty_title(locale, view),
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    special_view_empty_hint(locale, view),
                    Style::default().fg(theme.fg),
                )),
                Line::from(Span::styled(
                    special_view_empty_back_hint(locale, view),
                    Style::default().fg(theme.comment),
                )),
            ];
        }

        vec![
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(locale, "panel.native_title"),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(locale, "panel.native_hint"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(locale, "panel.native_focus"),
                Style::default().fg(theme.comment),
            )),
        ]
    }
}
mod file_tree {
    use crate::app::App;
    use ratatui::{
        layout::{Alignment, Rect},
        style::Style,
        widgets::{Block, Borders, Paragraph},
        Frame,
    };

    pub fn draw_file_tree(f: &mut Frame, app: &mut App, area: Rect) {
        if let Some(ref mut tree) = app.sidebar.file_tree {
            let theme = &app.theme;
            tree.render(f, area, theme);
        } else {
            let l = app.locale;
            let block = Block::default()
                .title(format!(" {} ", crate::i18n::t(l, "tree.explorer")))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
                .border_style(Style::default().fg(app.theme.border));
            let paragraph = Paragraph::new(crate::i18n::t(l, "tree.no_dir"))
                .block(block)
                .alignment(Alignment::Center);
            f.render_widget(paragraph, area);
        }
    }
}
mod folder_row {
    use super::metrics::{display_width, truncate_to_width};
    use super::style::{sidebar_folder_bg, sidebar_folder_fg};
    use crate::sidebar::SidebarFolderSummary;
    use ratatui::{
        style::{Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Cell, Row},
    };

    pub(crate) fn build_folder_row(
        folder: &SidebarFolderSummary,
        is_selected: bool,
        content_width: usize,
        theme: &crate::theme::Theme,
        is_expanded: bool,
        is_hovered: bool,
    ) -> Row<'static> {
        let is_minimal = content_width < 10;
        let card_bg = sidebar_folder_bg(is_selected, theme);
        let unread = folder.has_unread_stop;
        let icon = if is_expanded { "▾" } else { "▸" };
        let card_width = content_width.saturating_sub(2);

        let mut spans = Vec::new();
        spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
        spans.push(Span::styled(" ", Style::default().bg(card_bg)));

        let icon_style = Style::default()
            .fg(if is_selected {
                theme.accent
            } else if is_hovered {
                theme.border_focused
            } else {
                theme.accent
            })
            .bg(card_bg);
        spans.push(Span::styled(format!("{} ", icon), icon_style));

        if !is_minimal {
            let count = folder.thread_count.to_string();
            let count_width = display_width(&count);
            let label_width = card_width.saturating_sub(5 + count_width).clamp(1, 30);
            let label = truncate_to_width(&folder.label, label_width);
            let used_width = 1 + 2 + display_width(&label) + count_width;

            spans.push(Span::styled(
                label,
                folder_label_style(is_selected, unread, theme, card_bg),
            ));

            let fill_width = card_width.saturating_sub(used_width + 1);
            if fill_width > 0 {
                spans.push(Span::styled(
                    " ".repeat(fill_width),
                    Style::default().bg(card_bg),
                ));
            }

            spans.push(Span::styled(
                count,
                count_style(is_selected, unread, theme, card_bg),
            ));
        }

        spans.push(Span::styled(" ", Style::default().bg(card_bg)));
        spans.push(Span::styled(" ", Style::default().bg(theme.bg)));

        Row::new(vec![Cell::from(Text::from(vec![Line::from(spans)]))])
            .height(1)
            .style(Style::default().bg(theme.bg))
    }

    pub(super) fn folder_label_style(
        is_selected: bool,
        _unread: bool,
        theme: &crate::theme::Theme,
        card_bg: ratatui::style::Color,
    ) -> Style {
        Style::default()
            .fg(sidebar_folder_fg(is_selected, theme))
            .bg(card_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub(super) fn count_style(
        is_selected: bool,
        unread: bool,
        theme: &crate::theme::Theme,
        card_bg: ratatui::style::Color,
    ) -> Style {
        let mut style = Style::default()
            .fg(if is_selected {
                theme.highlight_fg
            } else {
                theme.accent
            })
            .bg(card_bg);
        if unread {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    }
}
mod labels {
    use crate::app::state::ThreadListView;
    use crate::i18n::Locale;

    pub(super) fn special_view_title_label(locale: Locale, view: ThreadListView) -> &'static str {
        match view {
            ThreadListView::Archived => {
                if is_cjk_locale(locale) {
                    "归档"
                } else {
                    "Archived"
                }
            }
            ThreadListView::Trash => {
                if is_cjk_locale(locale) {
                    "回收站"
                } else {
                    "Trash"
                }
            }
            ThreadListView::Normal => "",
        }
    }

    pub(super) fn display_scope_title_label(locale: Locale, live_only: bool) -> &'static str {
        if is_cjk_locale(locale) {
            if live_only {
                "在线"
            } else {
                "全部"
            }
        } else if live_only {
            "LIVE"
        } else {
            "ALL"
        }
    }

    pub(super) fn special_view_empty_title(locale: Locale, view: ThreadListView) -> &'static str {
        match view {
            ThreadListView::Archived => {
                if is_cjk_locale(locale) {
                    "没有归档会话"
                } else {
                    "No archived threads"
                }
            }
            ThreadListView::Trash => {
                if is_cjk_locale(locale) {
                    "回收站为空"
                } else {
                    "Trash is empty"
                }
            }
            ThreadListView::Normal => "",
        }
    }

    pub(super) fn special_view_empty_hint(locale: Locale, view: ThreadListView) -> &'static str {
        match view {
            ThreadListView::Archived => {
                if is_cjk_locale(locale) {
                    "当前没有可恢复的归档会话"
                } else {
                    "There are no archived threads to restore"
                }
            }
            ThreadListView::Trash => {
                if is_cjk_locale(locale) {
                    "还没有被 d 隐藏的线程"
                } else {
                    "No threads have been hidden with d yet"
                }
            }
            ThreadListView::Normal => "",
        }
    }

    pub(super) fn special_view_empty_back_hint(
        locale: Locale,
        view: ThreadListView,
    ) -> &'static str {
        match view {
            ThreadListView::Archived => {
                if is_cjk_locale(locale) {
                    "按 'Z' 返回普通视图"
                } else {
                    "Press 'Z' to return to the main view"
                }
            }
            ThreadListView::Trash => {
                if is_cjk_locale(locale) {
                    "从设置重新进入或按 Esc 退出特殊视图"
                } else {
                    "Re-open from Settings or press Esc to leave the special view"
                }
            }
            ThreadListView::Normal => "",
        }
    }

    fn is_cjk_locale(locale: Locale) -> bool {
        matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
    }
}
mod metrics {
    pub(crate) fn display_width(s: &str) -> usize {
        s.chars().map(char_display_width).sum()
    }

    pub(crate) fn char_display_width(c: char) -> usize {
        if c == '\t' {
            return 4;
        }
        if c.is_control() {
            return 0;
        }

        let code = c as u32;
        if matches!(
            code,
            0x1100..=0x115F
                | 0x2329..=0x232A
                | 0x2E80..=0xA4CF
                | 0xAC00..=0xD7A3
                | 0xF900..=0xFAFF
                | 0xFE10..=0xFE19
                | 0xFE30..=0xFE6F
                | 0xFF00..=0xFF60
                | 0xFFE0..=0xFFE6
                | 0x1F300..=0x1FAFF
                | 0x20000..=0x3FFFD
        ) {
            2
        } else {
            1
        }
    }

    pub(crate) fn truncate_to_width(text: &str, max_width: usize) -> String {
        if max_width == 0 {
            return String::new();
        }
        if display_width(text) <= max_width {
            return text.to_string();
        }

        let ellipsis = "…";
        let ellipsis_width = display_width(ellipsis);
        let target_width = max_width.saturating_sub(ellipsis_width);
        let mut result = String::new();
        let mut used = 0usize;

        for ch in text.chars() {
            let width = char_display_width(ch);
            if used + width > target_width {
                break;
            }
            result.push(ch);
            used += width;
        }

        result.push_str(ellipsis);
        result
    }
}
mod status {
    use crate::app::App;
    use ratatui::{
        layout::Rect,
        style::Style,
        widgets::{Block, Borders, Paragraph},
        Frame,
    };

    pub fn draw_agent_status_bar(f: &mut Frame, app: &App, area: Rect) {
        let l = app.locale;
        let active = app.panels.iter().filter(|p| p.is_active).count();
        let total = app.panels.len();
        let tmpl = crate::i18n::t(l, "panel.agent_count");
        let text = format!(
            " {} ",
            tmpl.replacen("{}", &total.to_string(), 1)
                .replacen("{}", &active.to_string(), 1)
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
            .border_style(Style::default().fg(app.theme.border));
        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, area);
    }
}
mod style {
    use crate::model::AgentType;
    use crate::theme::Theme;
    use ratatui::style::{Color, Modifier, Style};

    pub(crate) fn sidebar_card_bg(is_selected: bool, theme: &Theme) -> Color {
        if is_selected {
            blend_color(theme.border_focused, theme.highlight_bg, 0.18)
        } else {
            blend_color(theme.border, theme.bg, 0.16)
        }
    }

    pub(crate) fn sidebar_folder_bg(is_selected: bool, theme: &Theme) -> Color {
        if is_selected {
            blend_color(theme.border_focused, theme.highlight_bg, 0.18)
        } else {
            blend_color(theme.border, theme.bg, 0.14)
        }
    }

    pub(crate) fn sidebar_folder_fg(is_selected: bool, theme: &Theme) -> Color {
        if is_selected {
            theme.highlight_fg
        } else {
            theme.fg
        }
    }

    pub(crate) fn sidebar_thread_fg(is_selected: bool, theme: &Theme) -> Color {
        if is_selected {
            theme.highlight_fg
        } else {
            blend_color(theme.fg, theme.comment, 0.64)
        }
    }

    pub(crate) fn badge_color(agent_type: AgentType, theme: &Theme) -> Color {
        match agent_type {
            AgentType::Claude => Color::Rgb(249, 140, 87),
            AgentType::Codex => Color::Rgb(88, 166, 255),
            AgentType::Pi => Color::Rgb(100, 200, 160),
            AgentType::Grok => Color::Rgb(255, 70, 70),
            AgentType::Kimi => Color::Rgb(80, 200, 120),
            AgentType::Gemini => Color::Rgb(180, 140, 255),
            AgentType::OpenCode => Color::Rgb(250, 173, 20),
            AgentType::Aider => Color::Rgb(163, 190, 140),
            AgentType::Cursor => Color::Rgb(180, 140, 255),
            AgentType::Unknown => theme.comment,
        }
    }

    pub(crate) fn maybe_bold(style: Style, enabled: bool) -> Style {
        if enabled {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    }

    pub(crate) fn blend_color(highlight: Color, base: Color, mix: f32) -> Color {
        let mix = mix.clamp(0.0, 1.0);
        match (to_rgb(highlight), to_rgb(base)) {
            (Some((hr, hg, hb)), Some((br, bg, bb))) => Color::Rgb(
                blend_channel(hr, br, mix),
                blend_channel(hg, bg, mix),
                blend_channel(hb, bb, mix),
            ),
            _ if mix >= 0.5 => highlight,
            _ => base,
        }
    }

    fn blend_channel(highlight: u8, base: u8, mix: f32) -> u8 {
        let highlight = highlight as f32;
        let base = base as f32;
        (base + (highlight - base) * mix).round().clamp(0.0, 255.0) as u8
    }

    fn to_rgb(color: Color) -> Option<(u8, u8, u8)> {
        match color {
            Color::Black => Some((0, 0, 0)),
            Color::Red => Some((170, 0, 0)),
            Color::Green => Some((0, 170, 0)),
            Color::Yellow => Some((170, 85, 0)),
            Color::Blue => Some((0, 0, 170)),
            Color::Magenta => Some((170, 0, 170)),
            Color::Cyan => Some((0, 170, 170)),
            Color::Gray => Some((170, 170, 170)),
            Color::DarkGray => Some((85, 85, 85)),
            Color::LightRed => Some((255, 85, 85)),
            Color::LightGreen => Some((85, 255, 85)),
            Color::LightYellow => Some((255, 255, 85)),
            Color::LightBlue => Some((85, 85, 255)),
            Color::LightMagenta => Some((255, 85, 255)),
            Color::LightCyan => Some((85, 255, 255)),
            Color::White => Some((255, 255, 255)),
            Color::Rgb(r, g, b) => Some((r, g, b)),
            Color::Indexed(value) => Some((value, value, value)),
            Color::Reset => None,
        }
    }
}
mod thread_row;
mod viewport {
    use crate::sidebar::SidebarItem;
    use std::ops::Range;

    pub(crate) fn render_window<F>(
        len: usize,
        selected: Option<usize>,
        viewport_height: usize,
        mut row_height: F,
    ) -> Range<usize>
    where
        F: FnMut(usize) -> usize,
    {
        if len == 0 || viewport_height == 0 {
            return 0..0;
        }

        let selected = selected.unwrap_or(0).min(len - 1);
        let target_before = viewport_height / 2;
        let mut start = selected;
        let mut before = 0usize;

        while start > 0 {
            let height = row_height(start - 1).max(1);
            if before + height > target_before {
                break;
            }
            start -= 1;
            before += height;
        }

        let mut end = selected + 1;
        let mut used = before + row_height(selected).max(1);
        while end < len {
            let height = row_height(end).max(1);
            if used + height > viewport_height {
                break;
            }
            used += height;
            end += 1;
        }

        while start > 0 && used < viewport_height {
            let height = row_height(start - 1).max(1);
            if used + height > viewport_height {
                break;
            }
            start -= 1;
            used += height;
        }

        start..end
    }

    pub(crate) fn next_jump_badge_for_start(items: &[SidebarItem], start: usize) -> usize {
        items
            .iter()
            .take(start)
            .filter(|item| item.as_thread().is_some())
            .count()
            + 1
    }

    pub(crate) fn jump_badge_for_item(
        item: &SidebarItem,
        next_jump_badge: &mut usize,
    ) -> Option<usize> {
        item.as_thread()?;
        let badge = (*next_jump_badge <= 9).then_some(*next_jump_badge);
        *next_jump_badge += 1;
        badge
    }

    pub(crate) fn item_row_height(item: &SidebarItem) -> usize {
        match item {
            SidebarItem::Folder(_) => 1,
            SidebarItem::Thread(_) => 1,
        }
    }

    #[cfg(test)]
    pub(crate) fn visible_thread_jump_badges(items: &[SidebarItem]) -> Vec<Option<usize>> {
        let mut next_jump_badge = 1usize;
        items
            .iter()
            .map(|item| jump_badge_for_item(item, &mut next_jump_badge))
            .collect()
    }
}
mod width {
    use super::labels::{display_scope_title_label, special_view_title_label};
    use super::metrics;
    use crate::app::state::{PreferredPanelWidthCache, ThreadListView};
    use crate::app::App;
    use crate::sidebar::SidebarItem;

    const MIN_PANEL_WIDTH: u16 = 6;
    const MAX_PANEL_WIDTH: u16 = 90;
    const FOLDER_LABEL_WIDTH_LIMIT: usize = 40;
    const THREAD_TITLE_WIDTH_LIMIT: usize = 72;

    pub fn preferred_panel_width(app: &mut App) -> u16 {
        if app.sidebar.collapsed {
            return 0;
        }
        let thread_list_view = app.thread_list_view();
        let locale = app.locale;
        let live_only = app.showing_live_sessions();
        let manual_width = app.config.display.agent_panel_width;
        if !app.sidebar.visible_sidebar_items_dirty {
            if let Some(cache) = app.sidebar.preferred_panel_width_cache {
                if cache.locale == locale
                    && cache.thread_list_view == thread_list_view
                    && cache.live_only == live_only
                    && cache.manual_width == manual_width
                {
                    return cache.width;
                }
            }
        }

        let title_width = if thread_list_view != ThreadListView::Normal {
            metrics::display_width(&format!(
                " {} {} {} ",
                "○",
                special_view_title_label(locale, thread_list_view),
                88
            ))
        } else {
            metrics::display_width(&format!(
                " {} {} {} ",
                "○",
                display_scope_title_label(locale, live_only),
                888
            ))
        };
        let items = app.visible_sidebar_items_ref();
        let mut content_width = 10usize;
        for item in items {
            let item_width = match item {
                SidebarItem::Folder(folder) => {
                    2 + metrics::display_width(&metrics::truncate_to_width(
                        &folder.label,
                        FOLDER_LABEL_WIDTH_LIMIT,
                    ))
                }
                SidebarItem::Thread(thread) => thread_item_width(&thread.title),
            };
            content_width = content_width.max(item_width);
            if content_width >= MAX_PANEL_WIDTH as usize {
                break;
            }
        }
        let auto_width =
            (content_width.max(title_width) as u16 + 6).clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
        let width = manual_width
            .map(|manual| manual.clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH))
            .unwrap_or(auto_width);
        app.sidebar.preferred_panel_width_cache = Some(PreferredPanelWidthCache {
            width,
            locale,
            thread_list_view,
            live_only,
            manual_width,
        });
        width
    }

    pub(super) fn thread_item_width(title: &str) -> usize {
        let title_width =
            metrics::display_width(&metrics::truncate_to_width(title, THREAD_TITLE_WIDTH_LIMIT));
        9 + title_width
    }
}

#[cfg(test)]
pub(crate) mod tests;

pub use draw::draw_panel_list;
pub use file_tree::draw_file_tree;
pub use status::draw_agent_status_bar;
pub(crate) use viewport::item_row_height;
pub use width::preferred_panel_width;

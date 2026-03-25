use crate::app::state::FocusTarget;
use crate::app::App;
use crate::model::{AgentPanel, AgentState};
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const SHIMMER_SWEEP_SECONDS: f32 = 2.0;
const SHIMMER_PADDING: f32 = 10.0;
const SHIMMER_BAND_HALF_WIDTH: f32 = 5.0;

pub fn draw_panel_list(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let panel_is_focused = !app.show_tree && app.preview_focus == FocusTarget::Panel;
    let border_color = if panel_is_focused {
        theme.border_focused
    } else {
        theme.border
    };
    let focus_mark = if panel_is_focused { "●" } else { "○" };
    let title = format!(" {} {} ", focus_mark, app.filtered_panels().len());
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let (rows, is_empty) = {
        let filtered = app.filtered_panels();
        let is_empty = filtered.is_empty();
        let selected_idx = app.table_state.selected();
        let content_width = inner.width as usize;
        let rows: Vec<Row> = filtered
            .iter()
            .enumerate()
            .map(|(idx, panel)| {
                build_panel_row(
                    panel,
                    idx == selected_idx.unwrap_or(usize::MAX),
                    content_width,
                    theme,
                )
            })
            .collect();
        (rows, is_empty)
    };

    if is_empty {
        let empty_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_title"),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_hint"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_agents"),
                Style::default().fg(theme.accent),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_create"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_refresh"),
                Style::default().fg(theme.comment),
            )),
        ];
        let empty = Paragraph::new(empty_msg)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(empty, inner);
        return;
    }

    let table = Table::new(rows, [Constraint::Min(0)])
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Never);

    let mut table_state = app.table_state.clone();
    f.render_stateful_widget(table, inner, &mut table_state);
    app.table_state = table_state;
}

fn build_panel_row(
    panel: &AgentPanel,
    is_selected: bool,
    content_width: usize,
    theme: &crate::theme::Theme,
) -> Row<'static> {
    let row_bg = if is_selected {
        theme.highlight_bg
    } else {
        theme.bg
    };
    let surface_bg = effective_surface_bg(row_bg, is_selected, theme);
    let primary_color = if is_selected {
        theme.highlight_fg
    } else {
        theme.fg
    };
    let badge_color = badge_color(panel, theme);
    let dir_name = leaf_dir_name(&panel.working_dir);
    let label_width = content_width.saturating_sub(3).clamp(4, 15);
    let compact_path = truncate_to_width(&dir_name, label_width);
    let unread = panel.has_unread_stop;
    let dot_style = if panel.state == AgentState::Busy {
        breathing_badge_style(badge_color, surface_bg, row_bg)
    } else {
        maybe_bold(Style::default().fg(badge_color).bg(row_bg), unread)
    };
    let mut spans = vec![
        Span::styled("●".to_string(), dot_style),
        Span::styled(
            agent_letter(panel).to_string(),
            maybe_bold(
                Style::default()
                    .fg(if is_selected {
                        theme.highlight_fg
                    } else {
                        badge_color
                    })
                    .bg(row_bg),
                unread,
            ),
        ),
        Span::styled(" ".to_string(), Style::default().bg(row_bg)),
    ];
    let label_color = if panel.state == AgentState::Waiting && !is_selected {
        theme.success
    } else {
        primary_color
    };
    if panel.state == AgentState::Busy {
        spans.extend(shimmer_spans(
            &compact_path,
            label_color,
            surface_bg,
            row_bg,
        ));
    } else {
        spans.push(Span::styled(
            compact_path,
            maybe_bold(Style::default().fg(label_color).bg(row_bg), unread),
        ));
    }

    let line = Line::from(spans);
    Row::new(vec![Cell::from(Text::from(vec![line]))])
        .height(1)
        .style(Style::default().bg(row_bg))
}

fn badge_color(panel: &AgentPanel, theme: &crate::theme::Theme) -> ratatui::style::Color {
    match panel.agent_type {
        crate::model::AgentType::Claude => ratatui::style::Color::Rgb(249, 140, 87),
        crate::model::AgentType::Codex => ratatui::style::Color::Rgb(88, 166, 255),
        crate::model::AgentType::Kimi => ratatui::style::Color::Rgb(80, 200, 120),
        crate::model::AgentType::Gemini => ratatui::style::Color::Rgb(110, 168, 254),
        crate::model::AgentType::OpenCode => ratatui::style::Color::Rgb(250, 173, 20),
        crate::model::AgentType::Aider => ratatui::style::Color::Rgb(163, 190, 140),
        crate::model::AgentType::Cursor => ratatui::style::Color::Rgb(180, 140, 255),
        crate::model::AgentType::Unknown => theme.comment,
    }
}

fn effective_surface_bg(row_bg: Color, is_selected: bool, theme: &crate::theme::Theme) -> Color {
    match row_bg {
        Color::Reset if is_selected => theme.highlight_bg,
        Color::Reset => theme.border,
        _ => row_bg,
    }
}

fn agent_letter(panel: &AgentPanel) -> char {
    match panel.agent_type {
        crate::model::AgentType::Claude => 'C',
        crate::model::AgentType::Codex => 'X',
        crate::model::AgentType::Kimi => 'K',
        crate::model::AgentType::Gemini => 'G',
        crate::model::AgentType::OpenCode => 'O',
        crate::model::AgentType::Aider => 'A',
        crate::model::AgentType::Cursor => 'R',
        crate::model::AgentType::Unknown => '?',
    }
}

fn leaf_dir_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.to_string())
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
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

fn shimmer_spans(
    text: &str,
    base_color: Color,
    highlight_color: Color,
    bg: Color,
) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let total_width = display_width(text) as f32;
    let period = total_width + SHIMMER_PADDING * 2.0;
    let pos = (elapsed_since_start().as_secs_f32() % SHIMMER_SWEEP_SECONDS) / SHIMMER_SWEEP_SECONDS
        * period;
    let true_color = has_true_color();

    let mut spans = Vec::with_capacity(chars.len());
    let mut column = 0.0f32;

    for ch in chars {
        let width = char_display_width(ch).max(1) as f32;
        let center = column + (width * 0.5);
        let dist = ((center + SHIMMER_PADDING) - pos).abs();
        let intensity = if dist <= SHIMMER_BAND_HALF_WIDTH {
            let x = std::f32::consts::PI * (dist / SHIMMER_BAND_HALF_WIDTH);
            0.5 * (1.0 + x.cos())
        } else {
            0.0
        };
        let style = if true_color {
            let mixed = blend_color(highlight_color, base_color, intensity * 0.9);
            Style::default()
                .fg(mixed)
                .bg(bg)
                .add_modifier(Modifier::BOLD)
        } else {
            fallback_style(base_color, bg, intensity)
        };
        spans.push(Span::styled(ch.to_string(), style));
        column += width;
    }

    spans
}

fn breathing_badge_style(base_color: Color, surface_bg: Color, bg: Color) -> Style {
    let intensity = breathing_intensity();
    if has_true_color() {
        let style = Style::default()
            .fg(blend_color(base_color, surface_bg, 0.18 + intensity * 0.82))
            .bg(bg);
        if intensity >= 0.55 {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    } else {
        fallback_style(base_color, bg, intensity)
    }
}

fn breathing_intensity() -> f32 {
    let phase =
        (elapsed_since_start().as_secs_f32() % SHIMMER_SWEEP_SECONDS) / SHIMMER_SWEEP_SECONDS;
    0.5 * (1.0 - (phase * std::f32::consts::TAU).cos())
}

fn elapsed_since_start() -> Duration {
    static PROCESS_START: OnceLock<Instant> = OnceLock::new();
    PROCESS_START.get_or_init(Instant::now).elapsed()
}

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

fn fallback_style(base_color: Color, bg: Color, intensity: f32) -> Style {
    let style = Style::default().fg(base_color).bg(bg);
    if intensity >= 0.66 {
        style.add_modifier(Modifier::BOLD)
    } else if intensity <= 0.12 {
        style.add_modifier(Modifier::DIM)
    } else {
        style
    }
}

fn maybe_bold(style: Style, enabled: bool) -> Style {
    if enabled {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

fn blend_color(highlight: Color, base: Color, mix: f32) -> Color {
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

fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

fn char_display_width(c: char) -> usize {
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

pub fn preferred_panel_width(app: &App) -> u16 {
    let filtered = app.filtered_panels();
    let title_width = 6;
    let content_width = filtered
        .iter()
        .map(|panel| {
            let dir_name = leaf_dir_name(&panel.working_dir);
            let label_width = display_width(&truncate_to_width(&dir_name, 15));
            3 + label_width
        })
        .max()
        .unwrap_or(10);
    (content_width.max(title_width) as u16 + 4).clamp(12, 24)
}

pub fn draw_file_tree(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(ref mut tree) = app.file_tree {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shimmer_preserves_text_content() {
        let text = "rust-tui";
        let rendered: String = shimmer_spans(text, Color::White, Color::Cyan, Color::Black)
            .into_iter()
            .map(|span| span.content.to_string())
            .collect();
        assert_eq!(rendered, text);
    }

    #[test]
    fn preferred_panel_width_keeps_short_name_visible() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "kanban".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: crate::model::AgentType::Codex,
            working_dir: "/tmp/rust-tui".into(),
            is_active: true,
            state: AgentState::Busy,
            state_source: crate::model::AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });

        assert!(preferred_panel_width(&app) >= 13);
    }
}

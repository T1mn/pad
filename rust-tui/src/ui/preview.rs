use crate::app::state::FocusTarget;
use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use tui_markdown::{Options as MarkdownOptions, StyleSheet};

pub const PREVIEW_INFO_CARD_HEIGHT: u16 = 7;
pub const SESSION_CARD_HEIGHT: u16 = 4;

#[derive(Clone)]
struct PreviewMarkdownStyleSheet {
    theme: Theme,
}

impl PreviewMarkdownStyleSheet {
    fn new(theme: &Theme) -> Self {
        Self {
            theme: theme.clone(),
        }
    }
}

impl StyleSheet for PreviewMarkdownStyleSheet {
    fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::default()
                .fg(self.theme.keyword)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            2 => Style::default()
                .fg(self.theme.accent)
                .add_modifier(Modifier::BOLD),
            3 => Style::default()
                .fg(self.theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            _ => Style::default()
                .fg(self.theme.comment)
                .add_modifier(Modifier::ITALIC),
        }
    }

    fn code(&self) -> Style {
        inline_code_style(&self.theme)
    }

    fn link(&self) -> Style {
        Style::default()
            .fg(self.theme.accent)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn blockquote(&self) -> Style {
        Style::default().fg(self.theme.comment)
    }

    fn heading_meta(&self) -> Style {
        Style::default()
            .fg(self.theme.comment)
            .add_modifier(Modifier::DIM)
    }

    fn metadata_block(&self) -> Style {
        Style::default().fg(self.theme.comment)
    }
}

pub fn draw_preview(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme.clone();
    let l = app.locale;
    let preview_is_focused = app.preview_focus == FocusTarget::Preview;
    let focus_mark = if preview_is_focused { "●" } else { "○" };
    let title = format!(" {} {} ", focus_mark, crate::i18n::t(l, "preview.title"));

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(if preview_is_focused {
            theme.border_focused
        } else {
            theme.border
        }));

    // Empty state for preview
    if app.panels.is_empty() {
        let welcome = vec![
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.welcome"),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.subtitle"),
                Style::default().fg(theme.fg),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.keybindings"),
                Style::default().fg(theme.warning),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.nav_panels"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.attach"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.search"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.tree"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.create"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.delete"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.help"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.settings"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.quit"),
                Style::default().fg(theme.fg),
            )),
        ];
        let paragraph = Paragraph::new(welcome)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
        return;
    }

    let has_panel = app.selected_panel().is_some();
    if has_panel {
        let inner = block.inner(area);
        f.render_widget(block.clone(), area);

        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(PREVIEW_INFO_CARD_HEIGHT),
                Constraint::Min(0),
            ])
            .split(inner);

        if let Some(panel) = app.selected_panel() {
            draw_preview_info_card(f, app, split[0], &theme, panel);
        }

        if app.preview_source == crate::model::PreviewSource::Session
            && !app.preview_turns.is_empty()
        {
            draw_session_preview(f, app, split[1], &theme);
        } else {
            draw_plain_preview(f, app, split[1], false, &block, &theme);
        }
    } else {
        draw_plain_preview(f, app, area, true, &block, &theme);
    }
}

fn draw_preview_info_card(
    f: &mut Frame,
    app: &App,
    area: Rect,
    theme: &Theme,
    panel: &crate::model::AgentPanel,
) {
    let l = app.locale;
    let preview_source_label = match app.preview_source {
        crate::model::PreviewSource::Tmux => crate::i18n::t(l, "preview.source_tmux"),
        crate::model::PreviewSource::Session => crate::i18n::t(l, "preview.source_session"),
    };
    let cache_badge_label = if app.preview_source == crate::model::PreviewSource::Session
        && panel.session_cache_state == Some(crate::model::SessionCacheState::Cached)
    {
        Some(crate::i18n::t(l, "preview.session_cached"))
    } else {
        None
    };
    let status_label = localized_status_label(l, &panel.state);
    let status_color = match panel.state {
        crate::model::AgentState::Busy => theme.warning,
        crate::model::AgentState::Waiting => theme.success,
        crate::model::AgentState::Idle => theme.comment,
    };
    let branch = panel
        .git_info
        .as_ref()
        .and_then(|git| git.branch.as_ref())
        .map(|branch| branch.as_str());
    let git_text = if panel.git_info.is_some() {
        panel.git_display()
    } else {
        String::from("—")
    };
    let session_id = panel.agent_session_id.as_deref().unwrap_or("—");
    let label_width = 6usize;
    let header = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.highlight_bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.border));
    let inner = header.inner(area);
    f.render_widget(header, area);
    let (agent_badge_fg, agent_badge_bg) = preview_agent_badge_colors(&panel.agent_type, theme);

    let mut badge_spans = vec![
        preview_badge(
            &panel.agent_type.to_string().to_uppercase(),
            agent_badge_fg,
            agent_badge_bg,
        ),
        Span::raw(" "),
        preview_badge(preview_source_label, theme.bg, theme.accent),
        Span::raw(" "),
        preview_badge(status_label, theme.bg, status_color),
    ];
    badge_spans.push(Span::raw(" "));
    badge_spans.push(preview_badge(
        &format!("PID {}", panel.pid.as_deref().unwrap_or("?")),
        theme.fg,
        theme.bg,
    ));
    if let Some(label) = cache_badge_label {
        badge_spans.push(Span::raw(" "));
        badge_spans.push(preview_badge(label, theme.bg, theme.warning));
    }
    if let Some(branch) = branch {
        badge_spans.push(Span::raw(" "));
        badge_spans.push(preview_badge(
            &truncate_to_width(branch, 16),
            theme.fg,
            theme.bg,
        ));
    }

    let card = vec![
        Line::from(badge_spans),
        Line::from(vec![
            fixed_label("LOC", label_width, theme),
            Span::styled(
                truncate_to_width(
                    &format!("{}:{}.{}", panel.session, panel.window_index, panel.pane),
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("PATH", label_width, theme),
            Span::styled(
                truncate_to_width(
                    &panel.shortened_path(
                        inner.width.saturating_sub((label_width + 1) as u16) as usize
                    ),
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("GIT", label_width, theme),
            Span::styled(
                truncate_to_width(
                    &git_text,
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("SID", label_width, theme),
            Span::styled(
                truncate_to_width(
                    session_id,
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
    ];

    let paragraph =
        Paragraph::new(card).style(Style::default().bg(theme.highlight_bg).fg(theme.fg));
    f.render_widget(paragraph, inner);
}

fn draw_plain_preview(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    with_block: bool,
    block: &Block,
    theme: &Theme,
) {
    let viewport = if with_block { block.inner(area) } else { area };
    let scroll = resolve_preview_scroll(app, viewport);
    let lines: Vec<Line> = app
        .preview_content
        .lines()
        .map(|line| Line::from(format_line(line, theme)))
        .collect();

    let mut paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    if with_block {
        paragraph = paragraph.block(block.clone());
    }

    f.render_widget(paragraph, area);
}

fn draw_session_preview(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    if let Some(selected) = app.preview_expanded_turn {
        draw_session_detail(f, app, area, theme, selected);
    } else {
        draw_session_list(f, app, area, theme);
    }
}

fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let width = area.width.max(8) as usize;
    let mut lines: Vec<Line> = Vec::new();
    let mut selected_range = None;

    for (idx, turn) in app.preview_turns.iter().enumerate() {
        let start = lines.len();
        lines.extend(render_session_card(
            turn,
            idx == app.preview_selected_turn.unwrap_or(usize::MAX),
            width,
            theme,
        ));
        let end = lines.len().saturating_sub(1);
        if app.preview_selected_turn == Some(idx) {
            selected_range = Some((start, end));
        }
    }

    let scroll = resolve_session_list_scroll(app, selected_range, area.height, lines.len());
    let paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(theme.fg))
        .scroll((scroll, 0));
    f.render_widget(paragraph, area);
}

fn draw_session_detail(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme, selected: usize) {
    let Some(turn) = app.preview_turns.get(selected) else {
        return;
    };
    let lines = render_session_detail_lines(turn, area.width, theme);
    let scroll = resolve_preview_scroll_for_line_count(app, lines.len(), area.height);
    let paragraph = Paragraph::new(Text::from(lines)).scroll((scroll, 0));
    f.render_widget(paragraph, area);
}

fn render_session_detail_lines(
    turn: &crate::model::PreviewTurn,
    width: u16,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let canvas_width = width.max(12) as usize;
    let options = markdown_options(theme);
    let surface_bg = detail_surface(theme);

    let prompt =
        normalize_markdown_paragraph_spacing(&question_text_for_display(turn.question.trim()));
    let response = normalize_markdown_paragraph_spacing(&answer_text_for_display(
        turn.answer.as_deref().unwrap_or("...").trim(),
    ));

    let prompt_text = tui_markdown::from_str_with_options(&prompt, &options);
    let response_text = tui_markdown::from_str_with_options(&response, &options);

    let prompt_width = canvas_width.saturating_sub(4).max(1);
    let response_width = canvas_width.saturating_sub(4).max(1);
    let prompt_lines = wrap_text_to_width(
        &prompt_text,
        prompt_width,
        blend_color(
            fallback_color(theme.comment, theme.fg),
            fallback_color(theme.fg, theme.highlight_fg),
            0.55,
        ),
        surface_bg,
    );
    let response_lines = wrap_text_to_width(
        &response_text,
        response_width,
        fallback_color(theme.highlight_fg, theme.fg),
        surface_bg,
    );

    let mut lines = Vec::new();
    lines.push(render_detail_padding_line(canvas_width, surface_bg));
    for line in prompt_lines {
        lines.push(render_detail_content_line(line, prompt_width, surface_bg));
    }
    lines.push(render_detail_padding_line(canvas_width, surface_bg));
    lines.push(render_detail_separator_line(
        canvas_width,
        "Response",
        fallback_color(theme.bg, theme.highlight_bg),
        fallback_color(theme.border_focused, theme.accent),
        fallback_color(theme.border_focused, theme.accent),
        surface_bg,
    ));
    lines.push(render_detail_padding_line(canvas_width, surface_bg));
    for line in response_lines {
        lines.push(render_detail_content_line(line, response_width, surface_bg));
    }
    lines.push(render_detail_padding_line(canvas_width, surface_bg));
    lines
}

fn render_session_card(
    turn: &crate::model::PreviewTurn,
    selected: bool,
    width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let inner_width = width.saturating_sub(6).max(6);
    let q = truncate_to_width(
        &question_text_for_display(turn.question.trim()),
        inner_width,
    );
    let a = truncate_to_width(
        &answer_text_for_display(turn.answer.as_deref().unwrap_or("...").trim()),
        inner_width,
    );
    let block_bg = if selected {
        theme.highlight_bg
    } else {
        theme.bg
    };
    let marker_style = if selected {
        Style::default().fg(theme.border_focused).bg(block_bg)
    } else {
        Style::default().fg(theme.border).bg(block_bg)
    };
    let q_label_style = if selected {
        Style::default().fg(theme.highlight_fg).bg(block_bg)
    } else {
        Style::default().fg(theme.accent).bg(block_bg)
    };
    let a_label_style = if selected {
        Style::default().fg(theme.highlight_fg).bg(block_bg)
    } else {
        Style::default().fg(theme.success).bg(block_bg)
    };
    let text_style = if selected {
        Style::default()
            .fg(theme.highlight_fg)
            .bg(block_bg)
            .add_modifier(Modifier::DIM)
    } else {
        Style::default()
            .fg(theme.comment)
            .bg(block_bg)
            .add_modifier(Modifier::DIM)
    };
    vec![
        Line::from(vec![
            Span::styled("▌", marker_style),
            Span::styled(" Q ", q_label_style),
            Span::styled(q, text_style),
        ]),
        Line::from(vec![
            Span::styled("▌", marker_style),
            Span::styled(" A ", a_label_style),
            Span::styled(a, text_style),
        ]),
        Line::from(Span::styled(" ", Style::default().bg(block_bg))),
        Line::from(Span::styled(" ", Style::default().bg(block_bg))),
    ]
}

fn question_text_for_display(text: &str) -> String {
    strip_turn_prefix(text, &["Q:", "Q：", "Question:", "question:"]).to_string()
}

fn answer_text_for_display(text: &str) -> String {
    strip_turn_prefix(text, &["A:", "A：", "Answer:", "answer:"]).to_string()
}

fn localized_status_label(
    locale: crate::i18n::Locale,
    state: &crate::model::AgentState,
) -> &'static str {
    match state {
        crate::model::AgentState::Busy => crate::i18n::t(locale, "preview.working"),
        crate::model::AgentState::Waiting => crate::i18n::t(locale, "preview.waiting"),
        crate::model::AgentState::Idle => crate::i18n::t(locale, "preview.idle"),
    }
}

fn strip_turn_prefix<'a>(text: &'a str, prefixes: &[&str]) -> &'a str {
    let trimmed = text.trim();
    for prefix in prefixes {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.trim_start();
        }
    }
    trimmed
}

fn preview_badge(
    label: &str,
    fg: ratatui::style::Color,
    bg: ratatui::style::Color,
) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

fn preview_agent_badge_colors(
    agent_type: &crate::model::AgentType,
    theme: &Theme,
) -> (ratatui::style::Color, ratatui::style::Color) {
    match agent_type {
        crate::model::AgentType::Codex => (theme.bg, theme.border_focused),
        crate::model::AgentType::Claude => (theme.bg, Color::Rgb(249, 140, 87)),
        crate::model::AgentType::Gemini => (theme.bg, Color::Rgb(180, 140, 255)),
        crate::model::AgentType::Kimi | crate::model::AgentType::OpenCode => {
            (Color::White, Color::Black)
        }
        crate::model::AgentType::Aider => (theme.bg, theme.success),
        crate::model::AgentType::Cursor => (theme.bg, Color::Rgb(180, 140, 255)),
        crate::model::AgentType::Unknown => (theme.fg, theme.comment),
    }
}

fn fixed_label(label: &str, width: usize, theme: &Theme) -> Span<'static> {
    Span::styled(
        format!("{} ", pad_to_width(label, width)),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::BOLD),
    )
}

fn resolve_session_list_scroll(
    app: &mut App,
    selected_range: Option<(usize, usize)>,
    viewport_height: u16,
    total_lines: usize,
) -> u16 {
    if viewport_height == 0 {
        return 0;
    }
    let max_scroll = total_lines.saturating_sub(viewport_height as usize);
    let mut scroll = app
        .preview_list_scroll
        .min(max_scroll.min(u16::MAX as usize) as u16);

    if app.preview_follow_selection {
        if let Some((start, end)) = selected_range {
            let scroll_usize = scroll as usize;
            let viewport = viewport_height as usize;
            if start < scroll_usize {
                scroll = start.min(max_scroll).min(u16::MAX as usize) as u16;
            } else if end >= scroll_usize.saturating_add(viewport) {
                let adjusted = end
                    .saturating_add(1)
                    .saturating_sub(viewport)
                    .min(max_scroll)
                    .min(u16::MAX as usize);
                scroll = adjusted as u16;
            }
        }
    }

    app.preview_list_scroll = scroll;
    scroll
}

fn resolve_preview_scroll(app: &mut App, viewport: Rect) -> u16 {
    let content = app.preview_content.clone();
    resolve_preview_scroll_for_text(app, &content, viewport)
}

fn resolve_preview_scroll_for_line_count(
    app: &mut App,
    total_lines: usize,
    viewport_height: u16,
) -> u16 {
    if viewport_height == 0 {
        return 0;
    }
    let max_scroll = total_lines.saturating_sub(viewport_height as usize);
    let max_scroll = max_scroll.min(u16::MAX as usize) as u16;
    let scroll = if app.preview_follow_bottom {
        max_scroll
    } else {
        app.preview_scroll.min(max_scroll)
    };
    app.preview_scroll = scroll;
    scroll
}

fn resolve_preview_scroll_for_text(app: &mut App, content: &str, viewport: Rect) -> u16 {
    let max_scroll = precise_preview_max_scroll(content, viewport.width, viewport.height);
    let scroll = if app.preview_follow_bottom {
        max_scroll
    } else {
        app.preview_scroll.min(max_scroll)
    };
    app.preview_scroll = scroll;
    scroll
}

fn precise_preview_max_scroll(content: &str, viewport_width: u16, viewport_height: u16) -> u16 {
    if viewport_width == 0 || viewport_height == 0 {
        return 0;
    }

    let wrapped_rows = wrapped_row_count(content, viewport_width as usize);
    let max_scroll = wrapped_rows.saturating_sub(viewport_height as usize);
    max_scroll.min(u16::MAX as usize) as u16
}

fn wrapped_row_count(content: &str, viewport_width: usize) -> usize {
    if viewport_width == 0 {
        return 0;
    }

    let mut total = 0usize;
    for line in content.lines() {
        let width = display_width(line);
        let rows = if width == 0 {
            1
        } else {
            width.div_ceil(viewport_width)
        };
        total += rows.max(1);
    }

    if total == 0 {
        1
    } else {
        total
    }
}

fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

fn markdown_options(theme: &Theme) -> MarkdownOptions<PreviewMarkdownStyleSheet> {
    MarkdownOptions::new(PreviewMarkdownStyleSheet::new(theme))
}

fn detail_surface(theme: &Theme) -> Color {
    let base = fallback_color(theme.bg, theme.highlight_bg);
    let highlight = fallback_color(theme.highlight_bg, theme.border);
    blend_color(highlight, base, 0.52)
}

fn normalize_markdown_paragraph_spacing(text: &str) -> String {
    text.to_string()
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

fn pad_to_width(text: &str, target_width: usize) -> String {
    let width = display_width(text);
    if width >= target_width {
        return text.to_string();
    }

    let mut out = String::from(text);
    out.push_str(&" ".repeat(target_width - width));
    out
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

fn inline_code_style(theme: &Theme) -> Style {
    Style::default()
        .fg(derived_inline_code_fg(theme))
        .bg(derived_inline_code_bg(theme))
}

fn render_detail_separator_line(
    width: usize,
    label: &str,
    label_fg: Color,
    label_bg: Color,
    line_color: Color,
    surface_bg: Color,
) -> Line<'static> {
    let badge = preview_badge(label, label_fg, label_bg);
    let badge_width = display_width(badge.content.as_ref());
    let inner = width.saturating_sub(4);
    let gap = 2usize;
    let used = badge_width + gap;
    let left = inner.saturating_sub(used) / 2;
    let right = inner.saturating_sub(used + left);
    let line_style = Style::default().fg(line_color).bg(surface_bg);
    let fill = Style::default().bg(surface_bg);

    Line::from(vec![
        Span::styled("  ", fill),
        Span::styled("─".repeat(left), line_style),
        Span::styled(" ".repeat(gap / 2), fill),
        badge,
        Span::styled(" ".repeat(gap - gap / 2), fill),
        Span::styled("─".repeat(right), line_style),
        Span::styled("  ", fill),
    ])
}

fn render_detail_padding_line(width: usize, surface_bg: Color) -> Line<'static> {
    Line::from(Span::styled(
        " ".repeat(width),
        Style::default().bg(surface_bg),
    ))
}

fn render_detail_content_line(
    line: Line<'static>,
    content_width: usize,
    surface_bg: Color,
) -> Line<'static> {
    let used_width = line
        .spans
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum::<usize>()
        .min(content_width);
    let pad = content_width.saturating_sub(used_width);
    let fill = Style::default().bg(surface_bg);
    let mut spans = vec![Span::styled("  ", fill)];
    spans.extend(line.spans);
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), fill));
    }
    spans.push(Span::styled("  ", fill));
    Line::from(spans)
}

fn apply_surface_style(line: &Line<'_>, default_fg: Color, surface_bg: Color) -> Line<'static> {
    let spans = line
        .spans
        .iter()
        .map(|span| {
            let mut style = span.style;
            if style.fg.is_none() {
                style = style.fg(default_fg);
            }
            if style.bg.is_none() {
                style = style.bg(surface_bg);
            }
            Span::styled(span.content.to_string(), style)
        })
        .collect::<Vec<_>>();
    Line::from(spans)
}

fn wrap_text_to_width(
    text: &Text<'_>,
    width: usize,
    default_fg: Color,
    surface_bg: Color,
) -> Vec<Line<'static>> {
    let mut wrapped = Vec::new();
    for line in &text.lines {
        let styled = apply_surface_style(line, default_fg, surface_bg);
        wrapped.extend(wrap_styled_line(&styled, width));
    }

    if wrapped.is_empty() {
        wrapped.push(Line::default());
    }

    wrapped
}

fn wrap_styled_line(line: &Line<'_>, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![Line::default()];
    }
    if line.spans.is_empty() {
        return vec![Line::default()];
    }

    let mut lines = Vec::new();
    let mut current_spans = Vec::new();
    let mut current_width = 0usize;

    for span in &line.spans {
        let style = span.style;
        let mut buffer = String::new();

        for ch in span.content.chars() {
            let rendered = if ch == '\t' {
                "    ".to_string()
            } else {
                ch.to_string()
            };
            let ch_width = display_width(&rendered).max(1);

            if current_width > 0 && current_width + ch_width > width {
                if !buffer.is_empty() {
                    current_spans.push(Span::styled(std::mem::take(&mut buffer), style));
                }
                lines.push(Line::from(std::mem::take(&mut current_spans)));
                current_width = 0;
            }

            buffer.push_str(&rendered);
            current_width += ch_width;
        }

        if !buffer.is_empty() {
            current_spans.push(Span::styled(buffer, style));
        }
    }

    if current_spans.is_empty() {
        lines.push(Line::default());
    } else {
        lines.push(Line::from(current_spans));
    }

    lines
}

fn derived_inline_code_bg(theme: &Theme) -> Color {
    let base = fallback_color(theme.bg, theme.highlight_bg);
    let surface = fallback_color(theme.highlight_bg, theme.border);
    blend_color(surface, base, 0.72)
}

fn derived_inline_code_fg(theme: &Theme) -> Color {
    let base = fallback_color(theme.fg, theme.highlight_fg);
    let accent = fallback_color(theme.accent, base);
    blend_color(accent, base, 0.28)
}

fn fallback_color(primary: Color, fallback: Color) -> Color {
    match primary {
        Color::Reset => fallback,
        _ => primary,
    }
}

fn blend_color(highlight: Color, base: Color, mix: f32) -> Color {
    let mix = mix.clamp(0.0, 1.0);
    match (rgb_components(highlight), rgb_components(base)) {
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

fn rgb_components(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((255, 0, 0)),
        Color::Green => Some((0, 128, 0)),
        Color::Yellow => Some((255, 255, 0)),
        Color::Blue => Some((0, 0, 255)),
        Color::Magenta => Some((255, 0, 255)),
        Color::Cyan => Some((0, 255, 255)),
        Color::Gray => Some((128, 128, 128)),
        Color::DarkGray => Some((64, 64, 64)),
        Color::LightRed => Some((255, 102, 102)),
        Color::LightGreen => Some((144, 238, 144)),
        Color::LightYellow => Some((255, 255, 224)),
        Color::LightBlue => Some((173, 216, 230)),
        Color::LightMagenta => Some((255, 153, 255)),
        Color::LightCyan => Some((224, 255, 255)),
        Color::White => Some((255, 255, 255)),
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(_) | Color::Reset => None,
    }
}

fn tokenize_inline_code(text: &str, base_style: Style, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find('`') {
        let before = &rest[..start];
        if !before.is_empty() {
            spans.push(Span::styled(before.to_string(), base_style));
        }

        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            spans.push(Span::styled(rest.to_string(), base_style));
            return spans;
        };

        let code = &after_start[..end];
        if !code.is_empty() {
            spans.push(Span::styled(code.to_string(), inline_code_style(theme)));
        }
        rest = &after_start[end + 1..];
    }

    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_string(), base_style));
    }

    spans
}

fn retokenize_inline_code(spans: Vec<Span<'static>>, theme: &Theme) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    for span in spans {
        let content = span.content.into_owned();
        if content.contains('`') {
            out.extend(tokenize_inline_code(&content, span.style, theme));
        } else {
            out.push(Span::styled(content, span.style));
        }
    }
    out
}

fn format_line(line: &str, theme: &Theme) -> Vec<Span<'static>> {
    let stripped = line.trim();

    let user_markers = ["$", "#", "❯", ">", "%"];
    for marker in &user_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            let mut spans = vec![Span::styled(
                (*marker).to_string(),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(tokenize_inline_code(
                &format!(" {}", content),
                Style::default().fg(theme.success),
                theme,
            ));
            return spans;
        }
    }

    let ai_markers = ["●", "•", "💫", "🤖", "🟣", "🔵", "🟢", "⚡"];
    for marker in &ai_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            let mut spans = vec![Span::styled(
                (*marker).to_string(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(tokenize_inline_code(
                &format!(" {}", content),
                Style::default().fg(theme.accent),
                theme,
            ));
            return spans;
        }
    }

    if stripped.to_lowercase().contains("error") || stripped.to_lowercase().contains("failed") {
        return tokenize_inline_code(line, Style::default().fg(theme.error), theme);
    }

    if stripped.to_lowercase().contains("success")
        || stripped.to_lowercase().contains("done")
        || stripped.contains("✓")
    {
        return tokenize_inline_code(line, Style::default().fg(theme.success), theme);
    }

    tokenize_inline_code(line, Style::default(), theme)
}

#[cfg(test)]
mod tests {
    use super::{
        answer_text_for_display, derived_inline_code_bg, format_line, localized_status_label,
        normalize_markdown_paragraph_spacing, precise_preview_max_scroll,
        preview_agent_badge_colors, question_text_for_display, render_session_detail_lines,
        resolve_session_list_scroll,
    };
    use crate::app::App;
    use crate::model::{AgentType, PreviewTurn};
    use crate::theme::Theme;
    use ratatui::style::Color;

    #[test]
    fn bottom_scroll_accounts_for_wrapped_lines() {
        let content = "12345\n67890";
        let scroll = precise_preview_max_scroll(content, 3, 2);
        assert_eq!(scroll, 2);
    }

    #[test]
    fn short_content_does_not_scroll_past_top() {
        let content = "hi";
        let scroll = precise_preview_max_scroll(content, 20, 5);
        assert_eq!(scroll, 0);
    }

    #[test]
    fn cjk_width_counts_as_double_width() {
        let content = "你好\n世界";
        let scroll = precise_preview_max_scroll(content, 3, 1);
        assert_eq!(scroll, 3);
    }

    #[test]
    fn session_list_scroll_follows_selection_when_enabled() {
        let mut app = App::new();
        app.preview_follow_selection = true;
        app.preview_list_scroll = 0;

        let scroll = resolve_session_list_scroll(&mut app, Some((8, 11)), 4, 20);
        assert_eq!(scroll, 8);
        assert_eq!(app.preview_list_scroll, 8);
    }

    #[test]
    fn session_list_scroll_preserves_manual_scroll_when_follow_disabled() {
        let mut app = App::new();
        app.preview_follow_selection = false;
        app.preview_list_scroll = 6;

        let scroll = resolve_session_list_scroll(&mut app, Some((0, 3)), 4, 20);
        assert_eq!(scroll, 6);
        assert_eq!(app.preview_list_scroll, 6);
    }

    #[test]
    fn preview_display_strips_duplicate_role_prefixes() {
        assert_eq!(question_text_for_display("Q: how are you?"), "how are you?");
        assert_eq!(answer_text_for_display("A：all good"), "all good");
    }

    #[test]
    fn preview_display_preserves_plain_turn_text() {
        assert_eq!(
            question_text_for_display("plain question"),
            "plain question"
        );
        assert_eq!(answer_text_for_display("plain answer"), "plain answer");
    }

    #[test]
    fn preview_display_preserves_markdown_bullets() {
        assert_eq!(
            answer_text_for_display("- one\n  - two\n* three"),
            "- one\n  - two\n* three"
        );
    }

    #[test]
    fn preview_display_preserves_code_fence_bullets() {
        assert_eq!(
            answer_text_for_display("```text\n- keep\n```\n- keep"),
            "```text\n- keep\n```\n- keep"
        );
    }

    #[test]
    fn markdown_normalization_preserves_plain_hard_wraps() {
        assert_eq!(
            normalize_markdown_paragraph_spacing("line one\nline two"),
            "line one\nline two"
        );
    }

    #[test]
    fn markdown_normalization_preserves_setext_headings() {
        assert_eq!(
            normalize_markdown_paragraph_spacing("Title\n-----"),
            "Title\n-----"
        );
    }

    #[test]
    fn markdown_normalization_preserves_fenced_code_blocks() {
        assert_eq!(
            normalize_markdown_paragraph_spacing("```rust\nlet x = 1;\nlet y = 2;\n```\nafter"),
            "```rust\nlet x = 1;\nlet y = 2;\n```\nafter"
        );
    }

    #[test]
    fn inline_code_uses_theme_derived_background() {
        let theme = Theme::default();
        let spans = format_line("run `cargo check` now", &theme);
        let code_span = spans
            .iter()
            .find(|span| span.content == "cargo check")
            .unwrap();
        assert_eq!(code_span.style.bg, Some(derived_inline_code_bg(&theme)));
        assert_ne!(code_span.style.bg, Some(Color::Black));
    }

    #[test]
    fn preview_agent_badge_uses_requested_colors() {
        let theme = Theme::default();
        assert_eq!(
            preview_agent_badge_colors(&AgentType::Claude, &theme),
            (theme.bg, Color::Rgb(249, 140, 87))
        );
        assert_eq!(
            preview_agent_badge_colors(&AgentType::Gemini, &theme),
            (theme.bg, Color::Rgb(180, 140, 255))
        );
        assert_eq!(
            preview_agent_badge_colors(&AgentType::Kimi, &theme),
            (Color::White, Color::Black)
        );
        assert_eq!(
            preview_agent_badge_colors(&AgentType::OpenCode, &theme),
            (Color::White, Color::Black)
        );
    }

    #[test]
    fn session_detail_omits_prompt_label_and_keeps_response_divider() {
        let theme = Theme::default();
        let turn = PreviewTurn {
            question: "Q: first question".into(),
            answer: Some("A: answer body".into()),
        };
        let lines = render_session_detail_lines(&turn, 48, &theme);
        let rendered = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(rendered.contains("Response"));
        assert!(!rendered.contains("Prompt"));
        assert!(!rendered.contains("**Q**"));
        assert!(!rendered.contains("**A**"));
        assert!(!rendered.contains("---"));
    }

    #[test]
    fn session_detail_uses_horizontal_separator() {
        let theme = Theme::default();
        let turn = PreviewTurn {
            question: "plain question".into(),
            answer: Some("plain answer".into()),
        };
        let lines = render_session_detail_lines(&turn, 40, &theme);
        let rendered = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        assert!(rendered.iter().any(|line| line.contains("Response")));
        assert!(rendered.iter().any(|line| line.contains("──")));
        assert!(!rendered.iter().any(|line| line.starts_with("┌")));
        assert!(!rendered.iter().any(|line| line.starts_with("│")));
    }

    #[test]
    fn idle_status_badge_is_localized() {
        assert_eq!(
            localized_status_label(crate::i18n::Locale::ZhCN, &crate::model::AgentState::Idle),
            "空闲"
        );
    }
}

pub fn draw_file_preview(f: &mut Frame, app: &App, area: Rect) {
    use crate::tree::PreviewType;

    let theme = &app.theme;
    let l = app.locale;
    let title = if let Some(ref path) = app.file_preview_path {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let preview_type = PreviewType::from_path(path);
        let type_icon = match preview_type {
            PreviewType::Text => "📄",
            PreviewType::Markdown => "📝",
            PreviewType::Image => "🖼️",
            PreviewType::Directory => "📁",
            PreviewType::Binary => "📦",
            PreviewType::Unknown => "❓",
        };

        format!(" {} {} ", type_icon, file_name)
    } else {
        format!(" {} ", crate::i18n::t(l, "preview.file_title"))
    };

    let border_color = if app.mode == crate::app::state::Mode::FilePreview {
        theme.border_focused
    } else {
        theme.border
    };
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    // Markdown rendering via tui-markdown
    if let Some(ref path) = app.file_preview_path {
        let preview_type = PreviewType::from_path(path);
        if preview_type == PreviewType::Markdown {
            let normalized = normalize_markdown_paragraph_spacing(&app.file_preview_content);
            let options = markdown_options(theme);
            let text = tui_markdown::from_str_with_options(&normalized, &options);
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((app.file_preview_scroll, 0));
            f.render_widget(paragraph, area);
            return;
        }
    }

    let content = &app.file_preview_content;
    let lines: Vec<Line> = content
        .lines()
        .map(|line| Line::from(format_file_preview_line(line, theme)))
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.file_preview_scroll, 0));

    f.render_widget(paragraph, area);
}

fn format_file_preview_line(line: &str, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let trimmed = line.trim_start();
    let indent = line.chars().count() - trimmed.chars().count();

    if indent > 0 {
        spans.push(Span::raw(" ".repeat(indent)));
    }

    let stripped = trimmed;

    if let Some(idx) = stripped.find("//") {
        spans.push(Span::raw(stripped[..idx].to_string()));
        spans.push(Span::styled(
            stripped[idx..].to_string(),
            Style::default().fg(theme.comment),
        ));
        return retokenize_inline_code(spans, theme);
    }
    if stripped.starts_with('#') {
        spans.push(Span::styled(
            stripped.to_string(),
            Style::default().fg(theme.success),
        ));
        return retokenize_inline_code(spans, theme);
    }

    if stripped.contains('"') || stripped.contains('\'') {
        let mut in_string = false;
        let mut string_start = 0;

        for (i, c) in stripped.char_indices() {
            if c == '"' || c == '\'' {
                if !in_string {
                    if i > string_start {
                        spans.push(Span::raw(stripped[string_start..i].to_string()));
                    }
                    in_string = true;
                    string_start = i;
                } else {
                    spans.push(Span::styled(
                        stripped[string_start..=i].to_string(),
                        Style::default().fg(theme.string_color),
                    ));
                    in_string = false;
                    string_start = i + 1;
                }
            }
        }

        if string_start < stripped.len() {
            if in_string {
                spans.push(Span::styled(
                    stripped[string_start..].to_string(),
                    Style::default().fg(theme.string_color),
                ));
            } else {
                spans.push(Span::raw(stripped[string_start..].to_string()));
            }
        }

        if !spans.is_empty() {
            return retokenize_inline_code(spans, theme);
        }
    }

    let keywords = [
        "fn", "let", "mut", "if", "else", "for", "while", "match", "struct", "enum", "impl", "pub",
        "use", "mod", "const", "return", "true", "false", "None", "Some", "Ok", "Err",
    ];
    for kw in &keywords {
        if stripped.starts_with(kw)
            && (stripped.len() == kw.len()
                || !stripped[kw.len()..].starts_with(char::is_alphanumeric))
        {
            spans.push(Span::styled(
                (*kw).to_string(),
                Style::default().fg(theme.keyword),
            ));
            if stripped.len() > kw.len() {
                spans.push(Span::raw(stripped[kw.len()..].to_string()));
            }
            return retokenize_inline_code(spans, theme);
        }
    }

    if stripped
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        let end = stripped
            .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '_')
            .unwrap_or(stripped.len());
        spans.push(Span::styled(
            stripped[..end].to_string(),
            Style::default().fg(theme.number),
        ));
        if end < stripped.len() {
            spans.push(Span::raw(stripped[end..].to_string()));
        }
        return retokenize_inline_code(spans, theme);
    }

    tokenize_inline_code(line, Style::default(), theme)
}

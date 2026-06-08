use super::layout::centered_rect;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub(in crate::pad_sider::ui) fn draw_help(frame: &mut Frame) {
    let area = centered_rect(82, 72, frame.area());

    frame.render_widget(Clear, area);
    let paragraph = Paragraph::new(Text::from(help_lines()))
        .block(
            Block::default()
                .title(" help ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn help_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "pad-sider keys",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::default(),
        Line::from("Global"),
        Line::from("  F10           toggle fullscreen sider overlay"),
        Line::from("  ?             show / hide this help"),
        Line::from("  q             quit sider UI"),
        Line::from("  r             refresh project state"),
        Line::from("  Tab           switch Nav / Preview"),
        Line::from("  [ / ]         sider width: 45%-65%; extra space goes to preview"),
        Line::from("  n             toggle line numbers"),
        Line::from("  = / -         preview zoom"),
        Line::from("  t / c / p     focus Tree / Codex Runs / Preview"),
        Line::from("  II            switch Tree / Index Map"),
        Line::from("  g / G         top / bottom"),
        Line::default(),
        Line::from("Tree"),
        Line::from("  j/k ↑/↓       move / scroll focused area"),
        Line::from("  Enter/Space   expand or collapse directory"),
        Line::from("  Space         fullscreen preview selected file"),
        Line::from("  i             open nearest index.md guide"),
        Line::from("  /             fuzzy search files"),
        Line::default(),
        Line::from("Index Map"),
        Line::from("  II            focus all project index.md files"),
        Line::from("  Enter/Space   preview selected index.md"),
        Line::from("  o             reveal selected index.md in tree"),
        Line::default(),
        Line::from("Codex Runs"),
        Line::from("  c             show per-prompt Codex diffs"),
        Line::from("  j/k ↑/↓       select one Codex turn"),
        Line::from("  PgUp/PgDn     scroll selected diff"),
        Line::default(),
        Line::from("Preview"),
        Line::from("  q/Esc         close fullscreen preview"),
        Line::from("  j/k ↑/↓       scroll full or focused right preview"),
        Line::from("  PgUp/PgDn     scroll right preview"),
        Line::from("  g/G           top / bottom"),
        Line::from("  n · =/-       numbers · preview zoom"),
        Line::default(),
        Line::from("Search"),
        Line::from("  type          filter files"),
        Line::from("  Shift+Delete  clear filter"),
        Line::from("  ↑/↓           move"),
        Line::from("  Enter         reveal file in tree"),
        Line::from("  Esc           cancel"),
    ]
}

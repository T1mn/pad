use super::common::{mask_secret_prefix, render_modal_surface, truncate_modal_line};
use crate::app::App;
use crate::i18n::Locale;
use crate::theme::{ProviderConfig, Theme};
use crate::ui::layout::popup_area;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn draw_relay_settings(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;
    let agent_count = app.config.agents.len() as u16;
    let max_prov_count = app
        .config
        .agents
        .iter()
        .map(|a| a.providers.len())
        .max()
        .unwrap_or(1) as u16;
    let max_label = app
        .config
        .agents
        .iter()
        .flat_map(|a| {
            std::iter::once(a.name.len()).chain(a.providers.iter().map(|p| {
                let extra = if a.name == "codex" { 22 } else { 4 };
                p.label.len() + extra
            }))
        })
        .max()
        .unwrap_or(10) as u16;
    let content_w = (max_label.max(20).max(30) * 3 / 2) as u16;
    let content_h = (agent_count.max(max_prov_count).max(3) + 1) * 3 / 2;
    let area = popup_area(content_w, content_h, f.area());

    render_modal_surface(f, area, theme);

    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let title = match app.relay_view {
        crate::app::state::RelayView::AgentList => {
            format!(" {} ", crate::i18n::t(l, "relay.title"))
        }
        crate::app::state::RelayView::ProviderList | crate::app::state::RelayView::DetailPane => {
            format!(
                " {} [{}] ",
                crate::i18n::t(l, "relay.providers_label"),
                selected_agent.map(|a| a.name.as_str()).unwrap_or("?")
            )
        }
    };

    let block = Block::default()
        .title(title)
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

    match app.relay_view {
        crate::app::state::RelayView::AgentList => {
            let rows: Vec<Row> = app
                .config
                .agents
                .iter()
                .enumerate()
                .map(|(idx, agent)| {
                    let is_selected = idx == app.relay_selected_agent;
                    let active_label = agent
                        .active()
                        .map(|p| format!(" [{}]", p.label))
                        .unwrap_or_else(|| " [none]".to_string());
                    let style = if is_selected {
                        Style::default()
                            .bg(theme.highlight_bg)
                            .fg(theme.highlight_fg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg)
                    };
                    Row::new(vec![
                        Cell::from(format!("{}{}", agent.name, active_label)).style(style)
                    ])
                })
                .collect();
            let table = Table::new(rows, [Constraint::Min(0)]);
            f.render_widget(table, inner);
        }
        crate::app::state::RelayView::ProviderList => {
            let prov_rows: Vec<Row> = if let Some(agent) = selected_agent {
                if agent.providers.is_empty() {
                    vec![Row::new(vec![Cell::from(crate::i18n::t(l, "relay.empty"))])
                        .style(Style::default().fg(theme.comment))]
                } else {
                    agent
                        .providers
                        .iter()
                        .enumerate()
                        .map(|(i, prov)| {
                            let is_active = agent.active_provider == Some(i);
                            let selected = i == app.relay_selected_provider;
                            let active_marker = if is_active { "✓" } else { " " };
                            let dot_color = match prov.test_status {
                                Some(false) => Color::Rgb(180, 60, 60),
                                Some(true) => theme.success,
                                None => {
                                    if is_active {
                                        theme.success
                                    } else {
                                        theme.comment
                                    }
                                }
                            };
                            let style = if selected {
                                Style::default()
                                    .bg(theme.highlight_bg)
                                    .fg(if is_active {
                                        theme.success
                                    } else {
                                        theme.highlight_fg
                                    })
                                    .add_modifier(Modifier::BOLD)
                            } else if is_active {
                                Style::default()
                                    .fg(theme.success)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(theme.fg)
                            };
                            let mut spans = vec![
                                Span::styled(format!("{} ", active_marker), style),
                                Span::styled("● ", Style::default().fg(dot_color)),
                                Span::styled(prov.label.clone(), style),
                            ];
                            if agent.name == "codex" {
                                spans.extend(codex_provider_status_spans(prov, theme, selected));
                            }
                            spans.extend(provider_probe_summary_spans(prov, theme, selected));
                            Row::new(vec![Cell::from(Line::from(spans))])
                        })
                        .collect()
                }
            } else {
                vec![]
            };
            let prov_table = Table::new(prov_rows, [Constraint::Min(0)]);
            f.render_widget(prov_table, inner);
        }
        crate::app::state::RelayView::DetailPane => {
            let prov_rows: Vec<Row> = if let Some(agent) = selected_agent {
                agent
                    .providers
                    .iter()
                    .enumerate()
                    .map(|(i, prov)| {
                        let is_active = agent.active_provider == Some(i);
                        let marker = if is_active { "✓ " } else { "  " };
                        let style = Style::default().fg(theme.comment);
                        Row::new(vec![
                            Cell::from(format!("{}{}", marker, prov.label)).style(style)
                        ])
                    })
                    .collect()
            } else {
                vec![]
            };
            let prov_table = Table::new(prov_rows, [Constraint::Min(0)]);
            f.render_widget(prov_table, inner);
        }
    }

    let footer_text = if app.relay_editing {
        crate::i18n::t(l, "relay.footer_edit")
    } else {
        match app.relay_view {
            crate::app::state::RelayView::AgentList => crate::i18n::t(l, "relay.footer_agent"),
            crate::app::state::RelayView::ProviderList => {
                crate::i18n::t(l, "relay.footer_provider")
            }
            crate::app::state::RelayView::DetailPane => crate::i18n::t(l, "relay.footer_detail"),
        }
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

pub fn draw_relay_detail(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;

    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let prov = selected_agent.and_then(|a| a.providers.get(app.relay_selected_provider));
    let is_codex = selected_agent
        .map(|agent| agent.name.as_str() == "codex")
        .unwrap_or(false);
    let codex_preview_width = prov.map(codex_preview_width).unwrap_or(0) as u16;
    let content_w = if is_codex {
        codex_preview_width
            .max(78)
            .min(f.area().width.saturating_sub(6))
    } else {
        let max_field_len = prov
            .map(|prov| {
                prov.base_url
                    .len()
                    .max(prov.label.len())
                    .max(prov.api_key.len())
                    .max(20)
            })
            .unwrap_or(20) as u16;
        (max_field_len.max(30).max(40) * 8 / 5) as u16
    };
    let base_lines = if is_codex { 18u16 } else { 8u16 };
    let test_lines = if app.provider_test_in_progress {
        2
    } else if prov.map(|p| p.test_result.is_some()).unwrap_or(false) {
        4
    } else {
        0
    };
    let content_h = (base_lines + test_lines + 1) * 8 / 5;
    let area = popup_area(content_w, content_h, f.area());

    render_modal_surface(f, area, theme);

    let prov_label = prov.map(|p| p.label.as_str()).unwrap_or("?");
    let block = Block::default()
        .title(format!(
            " {} [{}] ",
            crate::i18n::t(l, "relay.details"),
            prov_label
        ))
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

    if let (Some(agent), Some(prov)) = (selected_agent, prov) {
        let field = app.relay_edit_field;
        let editing = app.relay_editing;
        let make_val = |idx: usize, val: &str| -> String {
            if editing && field == idx {
                format!("{}|", app.relay_edit_buffer)
            } else if val.is_empty() {
                "-".to_string()
            } else {
                val.to_string()
            }
        };
        let field_style = |idx: usize| -> Style {
            if field == idx {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            }
        };
        let masked_api_key = if editing && field == 2 {
            format!("{}|", app.relay_edit_buffer)
        } else if agent.name == "codex" {
            mask_secret_prefix(prov.codex_auth_token().as_deref().unwrap_or_default(), 10)
        } else if prov.api_key.is_empty() {
            "-".to_string()
        } else if prov.api_key.len() > 12 {
            format!("{}...", &prov.api_key[..12])
        } else {
            prov.api_key.clone()
        };

        if agent.name == "codex" {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(inner);
            let preview_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                .split(columns[1]);

            let mut form_lines = vec![
                Line::from(Span::styled(
                    crate::i18n::t(l, "relay.label"),
                    Style::default().fg(theme.comment),
                )),
                Line::from(Span::styled(make_val(0, &prov.label), field_style(0))),
                Line::from(""),
                Line::from(Span::styled(
                    crate::i18n::t(l, "relay.base_url"),
                    Style::default().fg(theme.comment),
                )),
                Line::from(Span::styled(make_val(1, &prov.base_url), field_style(1))),
                Line::from(""),
                Line::from(Span::styled(
                    crate::i18n::t(l, "relay.api_key"),
                    Style::default().fg(theme.comment),
                )),
                Line::from(Span::styled(masked_api_key, field_style(2))),
                Line::from(""),
                Line::from(Span::styled(
                    format!(
                        "{} {}",
                        crate::i18n::t(l, "relay.wire_api"),
                        prov.codex_wire_api()
                    ),
                    Style::default().fg(theme.comment),
                )),
            ];
            append_provider_test_lines(
                &mut form_lines,
                app.provider_test_in_progress,
                prov.test_status,
                prov.test_http_status,
                prov.test_latency_ms,
                prov.test_result.as_deref(),
                theme,
                l,
            );

            let form = Paragraph::new(form_lines)
                .block(
                    Block::default()
                        .title(" Fields ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.border)),
                )
                .wrap(Wrap { trim: false });
            f.render_widget(form, columns[0]);

            let auth_preview = Paragraph::new(
                codex_auth_preview_lines(prov)
                    .into_iter()
                    .map(|line| Line::from(Span::styled(line, Style::default().fg(theme.fg))))
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::default()
                    .title(" auth.json ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border)),
            )
            .wrap(Wrap { trim: false });
            f.render_widget(auth_preview, preview_split[0]);

            let config_preview = Paragraph::new(
                codex_config_preview_lines(prov)
                    .into_iter()
                    .map(|line| Line::from(Span::styled(line, Style::default().fg(theme.comment))))
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::default()
                    .title(" config.toml ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border)),
            )
            .wrap(Wrap { trim: false });
            f.render_widget(config_preview, preview_split[1]);
        } else {
            let mut detail_lines = vec![
                Line::from(Span::styled(
                    crate::i18n::t(l, "relay.label"),
                    Style::default().fg(theme.comment),
                )),
                Line::from(Span::styled(make_val(0, &prov.label), field_style(0))),
                Line::from(""),
                Line::from(Span::styled(
                    crate::i18n::t(l, "relay.base_url"),
                    Style::default().fg(theme.comment),
                )),
                Line::from(Span::styled(make_val(1, &prov.base_url), field_style(1))),
                Line::from(""),
                Line::from(Span::styled(
                    crate::i18n::t(l, "relay.api_key"),
                    Style::default().fg(theme.comment),
                )),
                Line::from(Span::styled(masked_api_key, field_style(2))),
            ];
            append_provider_test_lines(
                &mut detail_lines,
                app.provider_test_in_progress,
                prov.test_status,
                prov.test_http_status,
                prov.test_latency_ms,
                prov.test_result.as_deref(),
                theme,
                l,
            );
            let para = Paragraph::new(detail_lines).wrap(Wrap { trim: false });
            f.render_widget(para, inner);
        }
    } else {
        let para = Paragraph::new(vec![Line::from(Span::styled(
            crate::i18n::t(l, "relay.no_provider"),
            Style::default().fg(theme.comment),
        ))])
        .wrap(Wrap { trim: false });
        f.render_widget(para, inner);
    }

    let footer_text = if app.relay_editing {
        crate::i18n::t(l, "relay.footer_edit")
    } else {
        crate::i18n::t(l, "relay.footer_detail")
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

fn codex_auth_preview_lines(prov: &ProviderConfig) -> Vec<String> {
    vec![
        "{".to_string(),
        format!(
            "  \"OPENAI_API_KEY\": \"{}\"",
            mask_secret_prefix(prov.codex_auth_token().as_deref().unwrap_or_default(), 10)
        ),
        "}".to_string(),
    ]
}

fn codex_config_preview_lines(prov: &ProviderConfig) -> Vec<String> {
    let provider_name = prov.codex_provider_name();
    vec![
        format!("model_provider = \"{}\"", provider_name),
        String::new(),
        format!("[model_providers.{}]", provider_name),
        format!("name = \"{}\"", prov.label),
        format!("base_url = \"{}\"", prov.base_url),
        format!("wire_api = \"{}\"", prov.codex_wire_api()),
        "requires_openai_auth = true".to_string(),
    ]
}

fn codex_provider_status_spans(
    prov: &ProviderConfig,
    theme: &Theme,
    selected: bool,
) -> Vec<Span<'static>> {
    let auth_ready = prov.codex_auth_token().is_some();
    let config_ready = !prov.base_url.trim().is_empty();

    vec![
        Span::raw("  "),
        provider_status_badge("auth", auth_ready, theme, selected),
        Span::raw(" "),
        provider_status_badge("config", config_ready, theme, selected),
    ]
}

fn provider_status_badge(
    label: &'static str,
    ready: bool,
    theme: &Theme,
    selected: bool,
) -> Span<'static> {
    let fg = if ready { theme.success } else { theme.error };
    let bg = if selected {
        theme.highlight_bg
    } else {
        theme.bg
    };
    Span::styled(
        format!("[{}]", label),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

fn provider_probe_summary_spans(
    prov: &ProviderConfig,
    theme: &Theme,
    selected: bool,
) -> Vec<Span<'static>> {
    let Some(ok) = prov.test_status else {
        return Vec::new();
    };

    let mut spans = vec![Span::raw("  ")];
    let bg = if selected {
        theme.highlight_bg
    } else {
        theme.bg
    };

    if ok {
        if let Some(status) = prov.test_http_status {
            spans.push(Span::styled(
                format!("[{}]", status),
                Style::default()
                    .fg(http_status_color(status, theme))
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                "[ok]".to_string(),
                Style::default()
                    .fg(theme.success)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        if let Some(latency_ms) = prov.test_latency_ms {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("[{}ms]", latency_ms),
                Style::default()
                    .fg(latency_color(latency_ms, theme))
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    } else {
        spans.push(Span::styled(
            "[err]".to_string(),
            Style::default()
                .fg(theme.error)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ));
    }

    spans
}

fn codex_preview_width(prov: &ProviderConfig) -> usize {
    let preview_width = codex_auth_preview_lines(prov)
        .into_iter()
        .chain(codex_config_preview_lines(prov))
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        + 4;
    ((preview_width * 100) + 57) / 58 + 6
}

fn append_provider_test_lines(
    lines: &mut Vec<Line<'static>>,
    in_progress: bool,
    status: Option<bool>,
    http_status: Option<u16>,
    latency_ms: Option<u64>,
    result: Option<&str>,
    theme: &Theme,
    locale: Locale,
) {
    if !in_progress && status.is_none() && result.is_none() {
        return;
    }

    lines.push(Line::from(""));

    if in_progress {
        lines.push(Line::from(Span::styled(
            crate::i18n::t(locale, "relay.testing"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
        return;
    }

    let (label, color) = match status {
        Some(true) => ("Reachable", theme.success),
        Some(false) => ("Test Failed", theme.error),
        None => ("Test", theme.comment),
    };
    let mut summary_spans = vec![Span::styled(
        label,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )];
    if let Some(code) = http_status {
        summary_spans.push(Span::raw("  "));
        summary_spans.push(Span::styled(
            format!("HTTP {}", code),
            Style::default()
                .fg(http_status_color(code, theme))
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(latency_ms) = latency_ms {
        summary_spans.push(Span::raw("  "));
        summary_spans.push(Span::styled(
            format!("{} ms", latency_ms),
            Style::default()
                .fg(latency_color(latency_ms, theme))
                .add_modifier(Modifier::BOLD),
        ));
    }
    lines.push(Line::from(summary_spans));

    if let Some(result) = result {
        for line in result.lines().take(4) {
            lines.push(Line::from(Span::styled(
                truncate_modal_line(line, 72),
                Style::default().fg(theme.comment),
            )));
        }
    }
}

fn http_status_color(status: u16, theme: &Theme) -> Color {
    match status {
        100..=399 => theme.success,
        400..=499 => theme.warning,
        500..=599 => theme.error,
        _ => theme.comment,
    }
}

fn latency_color(latency_ms: u64, theme: &Theme) -> Color {
    match latency_ms {
        0..=800 => theme.success,
        801..=2500 => theme.warning,
        _ => theme.error,
    }
}

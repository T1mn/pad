use crate::app::state::SettingsDetailKind;
use crate::app::App;
use crate::i18n::{t, Locale};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

pub(in crate::ui::modals::settings) fn draw_simple_detail(
    f: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    value: String,
    body: Vec<String>,
    footer: &str,
) {
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title.to_string(),
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        header_area,
    );

    let mut lines = vec![
        Line::from(Span::styled(
            value,
            Style::default()
                .fg(app.theme.highlight_fg)
                .bg(app.theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
    ];
    lines.extend(body.into_iter().map(|line| {
        Line::from(Span::styled(
            line,
            Style::default()
                .fg(app.theme.comment)
                .add_modifier(Modifier::DIM),
        ))
    }));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body_area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            footer.to_string(),
            Style::default()
                .fg(app.theme.comment)
                .add_modifier(Modifier::DIM),
        ))),
        footer_area,
    );
}

pub(in crate::ui::modals::settings) fn simple_value_line(
    app: &App,
    kind: SettingsDetailKind,
) -> String {
    let locale = app.locale;
    match kind {
        SettingsDetailKind::AutoRefresh => {
            if app.config.auto_refresh {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            }
        }
        SettingsDetailKind::ClaudeFullAccess => {
            if app.config.agent_permissions.claude_auto_full_access {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            }
        }
        SettingsDetailKind::PreviewMode => match app.config.preview.mode.as_str() {
            "tmux" => t(locale, "settings.preview_mode_tmux").to_string(),
            "session" => t(locale, "settings.preview_mode_session").to_string(),
            _ => t(locale, "settings.preview_mode_auto").to_string(),
        },
        SettingsDetailKind::DisplayMode => match app.config.display.session_scope.as_str() {
            "all" => t(locale, "settings.display_mode_all").to_string(),
            _ => t(locale, "settings.display_mode_live").to_string(),
        },
        SettingsDetailKind::Trash => crate::thread_meta::deleted_thread_count()
            .unwrap_or_default()
            .to_string(),
        SettingsDetailKind::Version => env!("CARGO_PKG_VERSION").to_string(),
        _ => String::new(),
    }
}

pub(in crate::ui::modals::settings) fn detail_body_line(
    locale: Locale,
    kind: SettingsDetailKind,
) -> String {
    match (locale, kind) {
        (Locale::ZhCN, SettingsDetailKind::AutoRefresh) => "控制面板扫描是否自动刷新".to_string(),
        (Locale::ZhCN, SettingsDetailKind::ClaudeFullAccess) => {
            "启动时自动植入 bypassPermissions，并关闭 Claude sandbox".to_string()
        }
        (Locale::ZhCN, SettingsDetailKind::PreviewMode) => {
            "切换预览读取来源：自动 / tmux / session".to_string()
        }
        (Locale::ZhCN, SettingsDetailKind::DisplayMode) => {
            "切换只显示 live session 或显示全部 session".to_string()
        }
        (Locale::ZhCN, SettingsDetailKind::Trash) => {
            "打开 pad 回收站，查看或恢复被 d 隐藏的线程".to_string()
        }
        (Locale::ZhCN, SettingsDetailKind::Version) => "当前 pad 版本".to_string(),
        (_, SettingsDetailKind::AutoRefresh) => {
            "Controls whether pad refreshes scans automatically.".to_string()
        }
        (_, SettingsDetailKind::ClaudeFullAccess) => {
            "Apply bypassPermissions and disable Claude sandbox before launch.".to_string()
        }
        (_, SettingsDetailKind::PreviewMode) => {
            "Switch preview source between auto, tmux pane, and session transcript.".to_string()
        }
        (_, SettingsDetailKind::DisplayMode) => {
            "Switch between live-only sessions and all sessions.".to_string()
        }
        (_, SettingsDetailKind::Trash) => {
            "Open PAD trash to inspect or restore threads hidden by d.".to_string()
        }
        (_, SettingsDetailKind::Version) => "Current pad version.".to_string(),
        _ => String::new(),
    }
}

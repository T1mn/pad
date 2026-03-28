use super::common::{is_cjk_locale, render_modal_surface, truncate_modal_line_middle};
use crate::app::{App, ThreadActionKind, ThreadMetaEditKind};
use crate::i18n::Locale;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_delete_confirm(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let popup_width = 50;
    let popup_height = 8;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(
        area.x + popup_x,
        area.y + popup_y,
        popup_width,
        popup_height,
    );

    render_modal_surface(f, popup_area, theme);

    let panel_info = if let Some(ref panel) = app.sidebar.delete_target {
        format!("{}:{}.{}", panel.session, panel.window_index, panel.pane)
    } else {
        String::from("Unknown")
    };

    let text = format!(
        "{}\n\n{}\n\n{}\n{}",
        crate::i18n::t(l, "delete.confirm_msg"),
        panel_info,
        crate::i18n::t(l, "delete.yes_hint"),
        crate::i18n::t(l, "delete.cancel_hint")
    );

    let block = Block::default()
        .title(format!(" ⚠️ {} ", crate::i18n::t(l, "delete.title")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.error));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}

pub fn draw_thread_action_confirm(f: &mut Frame, app: &App, area: Rect) {
    if app.sidebar.thread_meta_editing {
        draw_thread_meta_editor(f, app, area);
        return;
    }

    let Some(action) = app.sidebar.pending_thread_action.as_ref() else {
        return;
    };

    let theme = &app.theme;
    let title = thread_action_modal_title(app.locale, action.kind);
    let subject = thread_action_subject(
        action.thread.title.as_str(),
        action.thread.session_id.as_deref(),
    );
    let confirm_line =
        thread_action_modal_confirm(app.locale, action.kind, action.thread.agent_type.clone());
    let warning =
        if action.kind == ThreadActionKind::Archive && action.thread.live_pane_id.is_some() {
            Some(thread_action_live_warning(
                app.locale,
                action.thread.agent_type.clone(),
            ))
        } else {
            None
        };

    let popup_width = 62;
    let popup_height = if warning.is_some() { 11 } else { 9 };
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(
        area.x + popup_x,
        area.y + popup_y,
        popup_width,
        popup_height,
    );

    render_modal_surface(f, popup_area, theme);

    let mut lines = vec![
        confirm_line.to_string(),
        String::new(),
        subject,
        String::new(),
    ];
    if let Some(warning) = warning {
        lines.push(warning.to_string());
        lines.push(String::new());
    }
    lines.push(thread_action_confirm_hint(app.locale).to_string());
    lines.push(thread_action_cancel_hint(app.locale).to_string());

    let block = Block::default()
        .title(format!(" ⚠ {} ", title))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.warning));

    let paragraph = Paragraph::new(lines.join("\n"))
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}

fn draw_thread_meta_editor(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let Some(thread) = app.sidebar.thread_meta_target.as_ref() else {
        return;
    };

    let title = thread_meta_editor_title(l, app.sidebar.thread_meta_edit_kind);
    let subject = thread_action_subject(thread.title.as_str(), thread.session_id.as_deref());
    let field_label = thread_meta_editor_field_label(l, app.sidebar.thread_meta_edit_kind);
    let help_text = thread_meta_editor_help_text(l);
    let prompt_text = thread_meta_editor_prompt_text(l, app.sidebar.thread_meta_edit_kind);

    let popup_width = 72;
    let popup_height = 10;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(
        area.x + popup_x,
        area.y + popup_y,
        popup_width,
        popup_height,
    );
    let subject_width = 34.min(popup_width.saturating_sub(4) as usize);
    let value_width = popup_width.saturating_sub(10) as usize;

    render_modal_surface(f, popup_area, theme);

    let subject = truncate_modal_line_middle(&subject, subject_width);
    let cursor_value = format!(
        "{}|",
        truncate_modal_line_middle(
            &app.sidebar.thread_meta_buffer,
            value_width.saturating_sub(1)
        )
    );
    let lines = vec![
        Line::from(Span::styled(
            subject,
            Style::default()
                .fg(theme.comment)
                .add_modifier(Modifier::DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            field_label,
            Style::default().fg(theme.comment),
        )),
        Line::from(Span::styled(
            cursor_value,
            Style::default()
                .fg(theme.highlight_fg)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            prompt_text,
            Style::default()
                .fg(theme.comment)
                .add_modifier(Modifier::DIM),
        )),
        Line::from(Span::styled(help_text, Style::default().fg(theme.comment))),
    ];

    let block = Block::default()
        .title(format!(" ✎ {} ", title))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}

pub fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let help_area = crate::ui::layout::popup_area(54, 30, area);

    render_modal_surface(f, help_area, theme);

    let block = Block::default()
        .title(format!(" ? {} ", crate::i18n::t(l, "help.title")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));

    let help_lines = vec![
        Line::from(Span::styled(
            crate::i18n::t(l, "app.title_full"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.nav"),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(crate::i18n::t(l, "help.move_down")),
        Line::from(crate::i18n::t(l, "help.move_up")),
        Line::from(crate::i18n::t(l, "help.jump")),
        Line::from(crate::i18n::t(l, "help.search_panels")),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.actions"),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(crate::i18n::t(l, "help.attach")),
        Line::from(crate::i18n::t(l, "help.create")),
        Line::from(crate::i18n::t(l, "help.delete")),
        Line::from(crate::i18n::t(l, "help.refresh")),
        Line::from(crate::i18n::t(l, "help.toggle_display_scope")),
        Line::from(crate::i18n::t(l, "help.focus_preview")),
        Line::from(crate::i18n::t(l, "help.select_preview")),
        Line::from(crate::i18n::t(l, "help.expand_preview")),
        Line::from(crate::i18n::t(l, "help.preview_back")),
        Line::from(crate::i18n::t(l, "help.scroll_preview")),
        Line::from(crate::i18n::t(l, "help.preview_home_end")),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.file_tree"),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(crate::i18n::t(l, "help.toggle_tree")),
        Line::from(crate::i18n::t(l, "help.tree_home")),
        Line::from(crate::i18n::t(l, "help.expand")),
        Line::from(crate::i18n::t(l, "help.go_up")),
        Line::from(crate::i18n::t(l, "help.scroll_file")),
        Line::from(crate::i18n::t(l, "help.scroll_file_page")),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.other"),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(crate::i18n::t(l, "help.f1")),
        Line::from(crate::i18n::t(l, "help.toggle_help")),
        Line::from(crate::i18n::t(l, "help.quit")),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.detach"),
            Style::default().fg(theme.comment),
        )),
    ];

    let paragraph = Paragraph::new(help_lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, help_area);
}

fn thread_action_modal_title(locale: Locale, kind: ThreadActionKind) -> &'static str {
    match (is_cjk_locale(locale), kind) {
        (true, ThreadActionKind::Archive) => "归档会话",
        (true, ThreadActionKind::Unarchive) => "恢复会话",
        (false, ThreadActionKind::Archive) => "Archive Thread",
        (false, ThreadActionKind::Unarchive) => "Restore Thread",
    }
}

fn thread_action_modal_confirm(
    locale: Locale,
    kind: ThreadActionKind,
    agent_type: crate::model::AgentType,
) -> String {
    match (is_cjk_locale(locale), kind, agent_type) {
        (true, ThreadActionKind::Archive, crate::model::AgentType::Codex) => {
            "确认归档这个 Codex 会话？".to_string()
        }
        (true, ThreadActionKind::Unarchive, crate::model::AgentType::Codex) => {
            "确认恢复这个 Codex 会话？".to_string()
        }
        (true, ThreadActionKind::Archive, crate::model::AgentType::Claude) => {
            "确认归档这个 Claude 会话？".to_string()
        }
        (true, ThreadActionKind::Unarchive, crate::model::AgentType::Claude) => {
            "确认恢复这个 Claude 会话？".to_string()
        }
        (true, ThreadActionKind::Archive, crate::model::AgentType::Gemini) => {
            "确认仅在 pad 侧归档这个 Gemini 会话？不会修改 ~/.gemini。".to_string()
        }
        (true, ThreadActionKind::Unarchive, crate::model::AgentType::Gemini) => {
            "确认从 pad 侧归档中恢复这个 Gemini 会话？不会修改 ~/.gemini。".to_string()
        }
        (false, ThreadActionKind::Archive, crate::model::AgentType::Codex) => {
            "Archive this Codex thread?".to_string()
        }
        (false, ThreadActionKind::Unarchive, crate::model::AgentType::Codex) => {
            "Restore this Codex thread?".to_string()
        }
        (false, ThreadActionKind::Archive, crate::model::AgentType::Claude) => {
            "Archive this Claude thread?".to_string()
        }
        (false, ThreadActionKind::Unarchive, crate::model::AgentType::Claude) => {
            "Restore this Claude thread?".to_string()
        }
        (false, ThreadActionKind::Archive, crate::model::AgentType::Gemini) => {
            "Archive this Gemini session in pad only? This does not modify ~/.gemini.".to_string()
        }
        (false, ThreadActionKind::Unarchive, crate::model::AgentType::Gemini) => {
            "Restore this Gemini session from pad archive? This does not modify ~/.gemini."
                .to_string()
        }
        (true, ThreadActionKind::Archive, _) => "确认归档这个会话？".to_string(),
        (true, ThreadActionKind::Unarchive, _) => "确认恢复这个会话？".to_string(),
        (false, ThreadActionKind::Archive, _) => "Archive this thread?".to_string(),
        (false, ThreadActionKind::Unarchive, _) => "Restore this thread?".to_string(),
    }
}

fn thread_action_live_warning(locale: Locale, agent_type: crate::model::AgentType) -> &'static str {
    match (is_cjk_locale(locale), agent_type) {
        (true, crate::model::AgentType::Codex) => {
            "这个会话仍然绑定 live pane。归档只会隐藏 pad 中的线程，并同步修改 Codex 的 sqlite/jsonl；不会关闭 tmux pane 或进程。"
        }
        (false, crate::model::AgentType::Codex) => {
            "This thread still has a live pane. Archiving only hides it in pad and updates Codex sqlite/jsonl. It does not kill the tmux pane or process."
        }
        (true, crate::model::AgentType::Claude) => {
            "这个会话仍然绑定 live pane。归档只会隐藏 pad 中的线程，并更新 pad 的 Claude sqlite 索引；不会关闭 tmux pane 或进程，也不会修改 ~/.claude 原始 jsonl。"
        }
        (false, crate::model::AgentType::Claude) => {
            "This thread still has a live pane. Archiving only hides it in pad and updates pad's Claude sqlite index. It does not kill the tmux pane or process, and it does not modify the original ~/.claude jsonl."
        }
        (true, crate::model::AgentType::Gemini) => {
            "这个会话仍然绑定 live pane。Pad 侧归档只会隐藏 pad 中的条目，不会修改 ~/.gemini，也不会关闭 tmux pane 或进程。"
        }
        (false, crate::model::AgentType::Gemini) => {
            "This thread still has a live pane. Pad-side archiving only hides it in pad. It does not modify ~/.gemini or kill the tmux pane/process."
        }
        (true, _) => "这个会话仍然绑定 live pane。归档不会关闭 tmux pane 或进程。",
        (false, _) => "This thread still has a live pane. Archiving does not kill the tmux pane or process.",
    }
}

fn thread_action_confirm_hint(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "按 'y' 确认"
    } else {
        "Press 'y' to confirm"
    }
}

fn thread_action_cancel_hint(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "任意键取消"
    } else {
        "Any other key cancels"
    }
}

fn thread_meta_editor_title(locale: Locale, kind: ThreadMetaEditKind) -> &'static str {
    match (is_cjk_locale(locale), kind) {
        (true, ThreadMetaEditKind::Title) => "编辑标题",
        (true, ThreadMetaEditKind::Tags) => "编辑标签",
        (false, ThreadMetaEditKind::Title) => "Edit Title",
        (false, ThreadMetaEditKind::Tags) => "Edit Tags",
    }
}

fn thread_meta_editor_field_label(locale: Locale, kind: ThreadMetaEditKind) -> &'static str {
    match (is_cjk_locale(locale), kind) {
        (true, ThreadMetaEditKind::Title) => "标题:",
        (true, ThreadMetaEditKind::Tags) => "标签（逗号分隔）:",
        (false, ThreadMetaEditKind::Title) => "Title:",
        (false, ThreadMetaEditKind::Tags) => "Tags (comma separated):",
    }
}

fn thread_meta_editor_prompt_text(locale: Locale, kind: ThreadMetaEditKind) -> &'static str {
    match (is_cjk_locale(locale), kind) {
        (true, ThreadMetaEditKind::Title) => "输入新的标题",
        (true, ThreadMetaEditKind::Tags) => "输入标签，多个标签用逗号分隔",
        (false, ThreadMetaEditKind::Title) => "Type a new title",
        (false, ThreadMetaEditKind::Tags) => "Type tags separated by commas",
    }
}

fn thread_meta_editor_help_text(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "Enter 保存 | Esc 取消 | 支持粘贴"
    } else {
        "Enter to save | Esc to cancel | Paste supported"
    }
}

fn thread_action_subject(title: &str, session_id: Option<&str>) -> String {
    let title = title.trim();
    if !title.is_empty() && title != "untitled" {
        title.to_string()
    } else {
        session_id.unwrap_or("unknown session").to_string()
    }
}

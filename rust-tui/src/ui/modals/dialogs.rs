mod delete {
    use super::super::common::render_modal_surface;
    use crate::app::App;
    use ratatui::{
        layout::{Alignment, Rect},
        style::Style,
        widgets::{Block, BorderType, Borders, Paragraph, Wrap},
        Frame,
    };

    pub fn draw_delete_confirm(f: &mut Frame, app: &App, area: Rect) {
        let theme = &app.theme;
        let l = app.locale;
        let popup_width = 62;
        let popup_height = 9;
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
}
mod help {
    use super::super::common::render_modal_surface;
    use crate::app::App;
    use ratatui::{
        layout::{Alignment, Rect},
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, BorderType, Borders, Paragraph, Wrap},
        Frame,
    };

    pub fn draw_help(f: &mut Frame, app: &App, area: Rect) {
        let theme = &app.theme;
        let l = app.locale;
        let help_area = crate::ui::layout::popup_area(68, 32, area);

        render_modal_surface(f, help_area, theme);

        let block = Block::default()
            .title(format!(" ? {} ", crate::i18n::t(l, "help.title")))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.accent));

        let paragraph = Paragraph::new(help_lines(app))
            .block(block)
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, help_area);
    }

    fn help_lines(app: &App) -> Vec<Line<'static>> {
        let theme = &app.theme;
        let l = app.locale;
        vec![
            Line::from(Span::styled(
                crate::i18n::t(l, "app.title_full"),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            section_title(crate::i18n::t(l, "help.nav"), theme.warning),
            Line::from(crate::i18n::t(l, "help.move_down")),
            Line::from(crate::i18n::t(l, "help.move_up")),
            Line::from(crate::i18n::t(l, "help.jump")),
            Line::from(crate::i18n::t(l, "help.search_panels")),
            Line::from(""),
            section_title(crate::i18n::t(l, "help.actions"), theme.warning),
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
            section_title(crate::i18n::t(l, "help.file_tree"), theme.warning),
            Line::from(crate::i18n::t(l, "help.toggle_tree")),
            Line::from(crate::i18n::t(l, "help.tree_home")),
            Line::from(crate::i18n::t(l, "help.expand")),
            Line::from(crate::i18n::t(l, "help.go_up")),
            Line::from(crate::i18n::t(l, "help.scroll_file")),
            Line::from(crate::i18n::t(l, "help.scroll_file_page")),
            Line::from(""),
            section_title(crate::i18n::t(l, "help.other"), theme.warning),
            Line::from(crate::i18n::t(l, "help.f1")),
            Line::from(crate::i18n::t(l, "help.toggle_help")),
            Line::from(crate::i18n::t(l, "help.quit")),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "help.detach"),
                Style::default().fg(theme.comment),
            )),
        ]
    }

    fn section_title(text: &'static str, color: ratatui::style::Color) -> Line<'static> {
        Line::from(Span::styled(
            text,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
    }
}
pub(crate) mod thread_action;
mod thread_meta {
    use super::super::common::{render_modal_surface, truncate_modal_line_middle};
    use super::thread_text::{
        thread_action_subject, thread_meta_editor_field_label, thread_meta_editor_help_text,
        thread_meta_editor_prompt_text, thread_meta_editor_title,
    };
    use crate::app::App;
    use ratatui::{
        layout::{Alignment, Rect},
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, BorderType, Borders, Paragraph, Wrap},
        Frame,
    };

    pub(super) fn draw_thread_meta_editor(f: &mut Frame, app: &App, area: Rect) {
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
        let paragraph = Paragraph::new(editor_lines(
            app,
            &subject,
            field_label,
            prompt_text,
            help_text,
            value_width,
        ))
        .block(editor_block(theme, title))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

        f.render_widget(paragraph, popup_area);
    }

    fn editor_lines<'a>(
        app: &'a App,
        subject: &'a str,
        field_label: &'static str,
        prompt_text: &'static str,
        help_text: &'static str,
        value_width: usize,
    ) -> Vec<Line<'a>> {
        let theme = &app.theme;
        let cursor_value = format!(
            "{}|",
            truncate_modal_line_middle(
                &app.sidebar.thread_meta_buffer,
                value_width.saturating_sub(1)
            )
        );
        vec![
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
        ]
    }

    fn editor_block(theme: &crate::theme::Theme, title: &'static str) -> Block<'static> {
        Block::default()
            .title(format!(" ✎ {} ", title))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.accent))
    }
}
mod thread_text;

pub use delete::draw_delete_confirm;
pub use help::draw_help;
pub use thread_action::draw_thread_action_confirm;

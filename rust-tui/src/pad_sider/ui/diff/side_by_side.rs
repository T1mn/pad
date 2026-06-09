use super::model::{DiffFile, DiffRow, DiffRowKind};
use super::styles::{add_style, delete_style, file_style, fit, hunk_style, meta_style, SEPARATOR};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::fmt::Write as _;

pub(super) fn render(files: &[DiffFile], width: usize, lines: &mut Vec<Line<'static>>) {
    let col_width = width.saturating_sub(SEPARATOR.len()).max(40) / 2;
    for file in files {
        lines.push(Line::from(Span::styled(
            format!("╭─ {}", file.title),
            file_style(),
        )));
        lines.push(side_line(
            "old",
            "new",
            col_width,
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::DarkGray),
        ));
        for meta in &file.meta {
            lines.push(Line::from(Span::styled(format!("│ {meta}"), meta_style())));
        }
        for hunk in &file.hunks {
            lines.push(Line::from(Span::styled(
                format!("│ {}", hunk.header),
                hunk_style(),
            )));
            for row in &hunk.rows {
                lines.push(render_row(row, col_width));
            }
        }
    }
}

fn render_row(row: &DiffRow, col_width: usize) -> Line<'static> {
    match row.kind {
        DiffRowKind::Context => side_line(
            &format_cell(row.old_no, &row.old_text, col_width),
            &format_cell(row.new_no, &row.new_text, col_width),
            col_width,
            Style::default(),
            Style::default(),
        ),
        DiffRowKind::Delete => side_line(
            &format_cell(row.old_no, &row.old_text, col_width),
            &format_cell(None, "", col_width),
            col_width,
            delete_style(),
            Style::default(),
        ),
        DiffRowKind::Add => side_line(
            &format_cell(None, "", col_width),
            &format_cell(row.new_no, &row.new_text, col_width),
            col_width,
            Style::default(),
            add_style(),
        ),
        DiffRowKind::Change => side_line(
            &format_cell(row.old_no, &row.old_text, col_width),
            &format_cell(row.new_no, &row.new_text, col_width),
            col_width,
            delete_style(),
            add_style(),
        ),
    }
}

fn side_line(
    left: &str,
    right: &str,
    col_width: usize,
    left_style: Style,
    right_style: Style,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(fit(left, col_width), left_style),
        Span::styled(SEPARATOR, Style::default().fg(Color::DarkGray)),
        Span::styled(fit(right, col_width), right_style),
    ])
}

fn format_cell(line: Option<usize>, text: &str, width: usize) -> String {
    let mut out = String::with_capacity(width);
    match line {
        Some(value) => {
            let _ = write!(out, "{value:>4}");
        }
        None => out.push_str("    "),
    }
    out.push_str(" │ ");

    let mut len = out.len();
    if len > width {
        out.truncate(width);
        return out;
    }

    for ch in text.chars().take(width - len) {
        out.push(ch);
        len += 1;
    }
    if len < width {
        out.extend(std::iter::repeat_n(' ', width - len));
    }
    out
}

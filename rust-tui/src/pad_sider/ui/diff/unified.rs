use super::model::{DiffFile, DiffRow, DiffRowKind};
use super::styles::{add_style, delete_style, file_style, hunk_style, meta_style};
use ratatui::{
    style::Style,
    text::{Line, Span},
};
use std::fmt::Write as _;

pub(super) fn render(files: &[DiffFile], lines: &mut Vec<Line<'static>>) {
    for file in files {
        lines.push(Line::from(Span::styled(
            format!("╭─ {}", file.title),
            file_style(),
        )));
        for meta in &file.meta {
            lines.push(Line::from(Span::styled(format!("│ {meta}"), meta_style())));
        }
        for hunk in &file.hunks {
            lines.push(Line::from(Span::styled(
                format!("│ {}", hunk.header),
                hunk_style(),
            )));
            for row in &hunk.rows {
                render_row(row, lines);
            }
        }
    }
}

fn render_row(row: &DiffRow, lines: &mut Vec<Line<'static>>) {
    match row.kind {
        DiffRowKind::Context => lines.push(unified_line(
            ' ',
            row.old_no,
            row.new_no,
            &row.new_text,
            Style::default(),
        )),
        DiffRowKind::Delete => lines.push(unified_line(
            '-',
            row.old_no,
            None,
            &row.old_text,
            delete_style(),
        )),
        DiffRowKind::Add => lines.push(unified_line(
            '+',
            None,
            row.new_no,
            &row.new_text,
            add_style(),
        )),
        DiffRowKind::Change => {
            lines.push(unified_line(
                '-',
                row.old_no,
                None,
                &row.old_text,
                delete_style(),
            ));
            lines.push(unified_line(
                '+',
                None,
                row.new_no,
                &row.new_text,
                add_style(),
            ));
        }
    }
}

fn unified_line(
    marker: char,
    old_no: Option<usize>,
    new_no: Option<usize>,
    text: &str,
    style: Style,
) -> Line<'static> {
    let mut out = String::with_capacity(12 + text.len());
    out.push(marker);
    out.push(' ');
    push_line_no(&mut out, old_no);
    out.push(' ');
    push_line_no(&mut out, new_no);
    out.push_str(" │ ");
    out.push_str(text);
    Line::from(Span::styled(out, style))
}

fn push_line_no(out: &mut String, value: Option<usize>) {
    match value {
        Some(value) => {
            let _ = write!(out, "{value:>4}");
        }
        None => out.push_str("    "),
    }
}

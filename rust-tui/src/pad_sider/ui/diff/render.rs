use super::model::{DiffFile, DiffRowKind};
use super::parse::parse_diff_document;
use super::styles::meta_style;
use ratatui::text::{Line, Span, Text};

const SIDE_BY_SIDE_MIN_WIDTH: u16 = 110;

pub fn render_diff_patch(content: &str, width: u16) -> Text<'static> {
    let doc = parse_diff_document(content);
    if doc.files.is_empty() {
        return Text::from(
            content
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect::<Vec<_>>(),
        );
    }

    let files = doc.files;
    let mut lines = Vec::with_capacity(doc.prelude.len() + rendered_file_rows(&files));
    lines.extend(
        doc.prelude
            .into_iter()
            .map(|line| Line::from(Span::styled(line, meta_style()))),
    );
    if width >= SIDE_BY_SIDE_MIN_WIDTH {
        super::side_by_side::render(&files, width as usize, &mut lines);
    } else {
        super::unified::render(&files, &mut lines);
    }
    Text::from(lines)
}

fn rendered_file_rows(files: &[DiffFile]) -> usize {
    files
        .iter()
        .map(|file| {
            1 + file.meta.len()
                + file
                    .hunks
                    .iter()
                    .map(|hunk| {
                        1 + hunk
                            .rows
                            .iter()
                            .map(|row| match row.kind {
                                DiffRowKind::Change => 2,
                                _ => 1,
                            })
                            .sum::<usize>()
                    })
                    .sum::<usize>()
        })
        .sum()
}

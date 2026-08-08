mod model {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(super) struct DiffDocument {
        pub prelude: Vec<String>,
        pub files: Vec<DiffFile>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(super) struct DiffFile {
        pub title: String,
        pub meta: Vec<String>,
        pub hunks: Vec<DiffHunk>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(super) struct DiffHunk {
        pub header: String,
        pub rows: Vec<DiffRow>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(super) struct DiffRow {
        pub old_no: Option<usize>,
        pub new_no: Option<usize>,
        pub old_text: String,
        pub new_text: String,
        pub kind: DiffRowKind,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum DiffRowKind {
        Context,
        Delete,
        Add,
        Change,
    }
}
mod parse;
mod render {
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
}
mod side_by_side;
mod styles {
    use ratatui::style::{Color, Modifier, Style};

    pub(super) const SEPARATOR: &str = " │ ";
    pub(super) const DELETE_BG: Color = Color::Rgb(52, 18, 18);
    pub(super) const ADD_BG: Color = Color::Rgb(18, 52, 24);

    pub(super) fn file_style() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub(super) fn hunk_style() -> Style {
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    }

    pub(super) fn meta_style() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub(super) fn delete_style() -> Style {
        Style::default().fg(Color::Red).bg(DELETE_BG)
    }

    pub(super) fn add_style() -> Style {
        Style::default().fg(Color::Green).bg(ADD_BG)
    }

    pub(super) fn fit(value: &str, width: usize) -> String {
        let mut out = String::with_capacity(width);
        let mut len = 0usize;
        for ch in value.chars().take(width) {
            out.push(ch);
            len += 1;
        }
        if len < width {
            out.extend(std::iter::repeat_n(' ', width - len));
        }
        out
    }
}
mod unified;

pub use render::render_diff_patch;

#[cfg(test)]
mod tests;

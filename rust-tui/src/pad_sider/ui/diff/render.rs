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

    let mut lines = doc
        .prelude
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), meta_style())))
        .collect::<Vec<_>>();
    if width >= SIDE_BY_SIDE_MIN_WIDTH {
        super::side_by_side::render(&doc.files, width as usize, &mut lines);
    } else {
        super::unified::render(&doc.files, &mut lines);
    }
    Text::from(lines)
}

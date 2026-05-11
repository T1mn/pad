use ratatui::text::{Line, Text};

pub fn apply_text_zoom(text: Text<'static>, zoom: i8) -> Text<'static> {
    if zoom < 0 {
        return compact(text);
    }
    if zoom > 0 {
        return roomy(text, zoom as usize);
    }
    text
}

fn compact(mut text: Text<'static>) -> Text<'static> {
    text.lines.retain(|line| !is_blank(line));
    text
}

fn roomy(text: Text<'static>, extra_blank_lines: usize) -> Text<'static> {
    let mut lines = Vec::with_capacity(text.lines.len() * (extra_blank_lines + 1));
    let original_len = text.lines.len();
    for (index, line) in text.lines.into_iter().enumerate() {
        let blank = is_blank(&line);
        lines.push(line);
        if !blank && index + 1 < original_len {
            lines.extend((0..extra_blank_lines).map(|_| Line::default()));
        }
    }
    Text::from(lines)
}

fn is_blank(line: &Line<'_>) -> bool {
    line.spans.iter().all(|span| span.content.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_removes_blank_lines() {
        let text = Text::from(vec![Line::from("one"), Line::default(), Line::from("two")]);
        assert_eq!(apply_text_zoom(text, -1).lines.len(), 2);
    }

    #[test]
    fn roomy_adds_blank_lines_between_content() {
        let text = Text::from(vec![Line::from("one"), Line::from("two")]);
        let zoomed = apply_text_zoom(text, 1);
        assert_eq!(zoomed.lines.len(), 3);
        assert!(is_blank(&zoomed.lines[1]));
    }
}

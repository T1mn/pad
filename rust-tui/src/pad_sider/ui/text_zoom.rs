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
#[path = "text_zoom_tests.rs"]
mod tests;

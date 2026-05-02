mod events;
mod render;
mod style;

pub use render::render_markdown;

#[cfg(test)]
mod tests {
    use super::render_markdown;

    fn line_texts(text: ratatui::text::Text<'_>) -> Vec<String> {
        text.lines
            .into_iter()
            .map(|line| {
                line.spans
                    .into_iter()
                    .map(|span| span.content.to_string())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn renders_blockquote_and_code_block_markers() {
        let lines = line_texts(render_markdown("> note\n\n```rs\nfn main() {}\n```"));
        assert!(lines.iter().any(|line| line.contains("│ note")));
        assert!(lines.iter().any(|line| line.contains(" code:rs ")));
        assert!(lines.iter().any(|line| line.contains("fn main() {}")));
    }

    #[test]
    fn renders_list_markers() {
        let lines = line_texts(render_markdown("- one\n- two"));
        assert!(lines.iter().any(|line| line.contains("• one")));
        assert!(lines.iter().any(|line| line.contains("• two")));
    }
}

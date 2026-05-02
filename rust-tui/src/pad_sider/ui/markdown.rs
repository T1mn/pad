mod events;
mod render;
mod style;

pub use render::render_markdown;

#[cfg(test)]
mod tests {
    use super::render_markdown;
    use ratatui::style::{Color, Modifier};

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

    #[test]
    fn inline_code_uses_color_without_background() {
        let text = render_markdown("run `cargo test` now");
        let span = text
            .lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content == "cargo test")
            .expect("inline code span");

        assert_eq!(span.style.fg, Some(Color::Yellow));
        assert_eq!(span.style.bg, None);
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn code_block_uses_color_without_background() {
        let text = render_markdown("```rs\nfn main() {}\n```");
        let spans = text
            .lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .collect::<Vec<_>>();

        let label = spans
            .iter()
            .find(|span| span.content.contains("code:rs"))
            .expect("code block label");
        let code = spans
            .iter()
            .find(|span| span.content.contains("fn main"))
            .expect("code block line");

        assert_eq!(label.style.bg, None);
        assert_eq!(code.style.bg, None);
    }
}

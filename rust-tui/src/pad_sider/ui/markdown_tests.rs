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
fn renders_blockquote_and_hides_code_block_language() {
    let lines = line_texts(render_markdown("> note\n\n```rs\nfn main() {}\n```"));
    assert!(lines.iter().any(|line| line.contains("│ note")));
    assert!(!lines.iter().any(|line| line.contains("code:rs")));
    assert!(lines.iter().any(|line| line.contains("fn main() {}")));
}

#[test]
fn renders_list_markers() {
    let lines = line_texts(render_markdown("- one\n- two"));
    assert!(lines.iter().any(|line| line.contains("• one")));
    assert!(lines.iter().any(|line| line.contains("• two")));
}

#[test]
fn renders_without_extra_block_spacing() {
    let lines = line_texts(render_markdown("# Title\n\nbody\n\n- one\n- two"));
    assert_eq!(lines, vec!["Title", "body", "• one", "• two"]);
}

#[test]
fn preserves_blank_lines_inside_code_blocks() {
    let lines = line_texts(render_markdown("```rs\nlet a = 1;\n\nlet b = 2;\n```"));
    assert!(lines.iter().any(|line| line.contains("let a = 1;")));
    assert!(lines.iter().any(|line| line.trim().is_empty()));
    assert!(lines.iter().any(|line| line.contains("let b = 2;")));
}

#[test]
fn inline_code_uses_distinct_background() {
    let text = render_markdown("run `cargo test` now");
    let span = text
        .lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .find(|span| span.content == "cargo test")
        .expect("inline code span");

    assert_eq!(span.style.fg, Some(Color::Rgb(224, 175, 104)));
    assert_eq!(span.style.bg, Some(Color::Rgb(42, 43, 61)));
    assert!(span.style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn code_block_uses_distinct_background() {
    let text = render_markdown("```rs\nfn main() {}\n```");
    let spans = text
        .lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .collect::<Vec<_>>();

    let code = spans
        .iter()
        .find(|span| span.content.contains("fn main"))
        .expect("code block line");

    assert_eq!(code.style.fg, Some(Color::Rgb(255, 158, 100)));
    assert_eq!(code.style.bg, Some(Color::Rgb(26, 27, 38)));
}

#[test]
fn code_block_language_changes_color() {
    let bash = render_markdown("```bash\necho ok\n```");
    let python = render_markdown("```python\nprint('ok')\n```");
    let bash_span = bash.lines[0]
        .spans
        .iter()
        .find(|span| span.content.contains("echo"))
        .unwrap();
    let python_span = python.lines[0]
        .spans
        .iter()
        .find(|span| span.content.contains("print"))
        .unwrap();

    assert_eq!(bash_span.style.fg, Some(Color::Rgb(158, 206, 106)));
    assert_eq!(python_span.style.fg, Some(Color::Rgb(122, 162, 247)));
}

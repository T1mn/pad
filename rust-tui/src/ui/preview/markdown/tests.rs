mod inline {
    use super::super::inline::format_line;
    use crate::theme::Theme;

    #[test]
    fn format_line_detects_error_case_insensitively() {
        let theme = Theme::default();
        let spans = format_line("FAILED to run", &theme);

        assert_eq!(spans[0].style.fg, Some(theme.error));
    }

    #[test]
    fn format_line_detects_success_case_insensitively() {
        let theme = Theme::default();
        let spans = format_line("SUCCESS", &theme);

        assert_eq!(spans[0].style.fg, Some(theme.success));
    }
}

mod normalize {
    use super::super::normalize_session_detail_markdown;

    #[test]
    fn inserts_paragraph_gaps_between_plain_lines() {
        assert_eq!(
            normalize_session_detail_markdown("first\nsecond"),
            "first\n\nsecond"
        );
    }

    #[test]
    fn keeps_fenced_code_lines_together() {
        assert_eq!(
            normalize_session_detail_markdown("```rs\nlet a = 1;\nlet b = 2;\n```"),
            "```rs\nlet a = 1;\nlet b = 2;\n```"
        );
    }
}

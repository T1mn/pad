use super::{
    build_session_list_lines, render_session_card, render_session_detail_lines,
    session_list_total_lines, session_turn_index_at_line,
};
use crate::model::PreviewTurn;
use crate::theme::Theme;
use ratatui::style::Modifier;

#[test]
fn selected_range_excludes_gap_line() {
    let turns = vec![
        PreviewTurn {
            question: "first".into(),
            answer: Some("one".into()),
        },
        PreviewTurn {
            question: "second".into(),
            answer: Some("two".into()),
        },
    ];

    let (lines, selected_range) = build_session_list_lines(&turns, Some(0), 40, &Theme::default());
    assert_eq!(lines.len(), 7);
    assert_eq!(selected_range, Some((0, 2)));
}

#[test]
fn gap_line_has_no_turn_hit_target() {
    assert_eq!(session_list_total_lines(2), 7);
    assert_eq!(session_turn_index_at_line(0, 2), Some(0));
    assert_eq!(session_turn_index_at_line(1, 2), Some(0));
    assert_eq!(session_turn_index_at_line(2, 2), Some(0));
    assert_eq!(session_turn_index_at_line(3, 2), None);
    assert_eq!(session_turn_index_at_line(4, 2), Some(1));
    assert_eq!(session_turn_index_at_line(5, 2), Some(1));
    assert_eq!(session_turn_index_at_line(6, 2), Some(1));
    assert_eq!(session_turn_index_at_line(7, 2), None);
}

#[test]
fn session_card_renders_two_answer_lines() {
    let theme = Theme::default();
    let lines = render_session_card(
        &PreviewTurn {
            question: "question".into(),
            answer: Some("answer line one answer line two".into()),
        },
        false,
        20,
        &theme,
    );

    assert_eq!(lines.len(), 3);
    assert!(lines[1]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>()
        .contains("answer line"));
    assert!(!lines[2]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>()
        .trim()
        .is_empty());

    let body_span = &lines[1].spans[2];
    assert_eq!(body_span.style.fg, Some(theme.fg));
    assert!(!body_span.style.add_modifier.contains(Modifier::DIM));
}

#[test]
fn session_detail_prompt_uses_primary_text_color() {
    let theme = Theme::default();
    let lines = render_session_detail_lines(
        &PreviewTurn {
            question: "question".into(),
            answer: Some("answer".into()),
        },
        40,
        &theme,
    );

    let prompt_span = lines[1]
        .spans
        .iter()
        .find(|span| span.content.contains("question"))
        .expect("prompt text span");
    assert_eq!(prompt_span.style.fg, Some(theme.fg));
}

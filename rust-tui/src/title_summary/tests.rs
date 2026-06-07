use super::{normalize_generated_title, select_turn_window, should_refresh_title, SummaryWireApi};
use crate::model::PreviewTurn;

#[test]
fn responses_is_default_wire_api() {
    assert_eq!(SummaryWireApi::from_config(""), SummaryWireApi::Responses);
    assert_eq!(
        SummaryWireApi::from_config("responses"),
        SummaryWireApi::Responses
    );
    assert_eq!(SummaryWireApi::from_config("chat"), SummaryWireApi::Chat);
}

#[test]
fn title_refresh_triggers_after_initial_threshold() {
    assert!(!should_refresh_title(2, None));
    assert!(should_refresh_title(3, None));
    assert!(should_refresh_title(11, None));
    assert!(!should_refresh_title(8, Some(3)));
    assert!(should_refresh_title(9, Some(3)));
}

#[test]
fn initial_window_uses_three_turns_in_chronological_order() {
    let turns = vec![
        PreviewTurn {
            question: "third".into(),
            answer: Some("c".into()),
        },
        PreviewTurn {
            question: "second".into(),
            answer: Some("b".into()),
        },
        PreviewTurn {
            question: "first".into(),
            answer: Some("a".into()),
        },
    ];

    let selected = select_turn_window(&turns, None);
    assert_eq!(selected.len(), 3);
    assert_eq!(selected[0].question, "first");
    assert_eq!(selected[2].question, "third");
}

#[test]
fn refresh_window_keeps_six_newest_turns() {
    let turns = (1..=8)
        .rev()
        .map(|idx| PreviewTurn {
            question: format!("q{idx}"),
            answer: None,
        })
        .collect::<Vec<_>>();

    let selected = select_turn_window(&turns, Some(3));
    assert_eq!(selected.len(), 6);
    assert_eq!(selected[0].question, "q3");
    assert_eq!(selected[5].question, "q8");
}

#[test]
fn title_normalization_trims_wrappers_and_prefixes() {
    assert_eq!(
        normalize_generated_title("Title: \"Refactor tmux popup flow\"").as_deref(),
        Some("Refactor tmux popup flow")
    );
    assert_eq!(
        normalize_generated_title("《修复会话标题自动摘要》").as_deref(),
        Some("修复会话标题自动摘要")
    );
}

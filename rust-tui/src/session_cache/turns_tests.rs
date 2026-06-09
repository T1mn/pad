use super::{normalize_cached_codex_prompt, normalize_turns};
use crate::model::PreviewTurn;

#[test]
fn normalize_turns_matches_for_owned_and_borrowed_inputs() {
    let turns = vec![
        PreviewTurn {
            question: "  hello  ".to_string(),
            answer: Some("  world  ".to_string()),
        },
        PreviewTurn {
            question: "   ".to_string(),
            answer: Some("drop".to_string()),
        },
    ];

    let from_owned = normalize_turns(turns.clone(), false);
    let from_borrowed = normalize_turns(&turns, false);

    assert_eq!(from_owned, from_borrowed);
    assert_eq!(
        from_owned,
        vec![PreviewTurn {
            question: "hello".to_string(),
            answer: Some("world".to_string()),
        }]
    );
}

#[test]
fn normalize_turns_stops_after_history_limit_valid_turns() {
    let mut turns = vec![PreviewTurn {
        question: "   ".to_string(),
        answer: Some("drop".to_string()),
    }];
    for idx in 0..(crate::session_cache::SESSION_HISTORY_TURN_LIMIT + 5) {
        turns.push(PreviewTurn {
            question: format!(" q{idx} "),
            answer: None,
        });
    }

    let normalized = normalize_turns(&turns, false);

    assert_eq!(
        normalized.len(),
        crate::session_cache::SESSION_HISTORY_TURN_LIMIT
    );
    assert_eq!(normalized[0].question, "q0");
    assert_eq!(
        normalized.last().map(|turn| turn.question.as_str()),
        Some("q49")
    );
}

#[test]
fn normalize_cached_codex_prompt_trims_without_precopy() {
    assert_eq!(
        normalize_cached_codex_prompt(Some("  hello  "), false).as_deref(),
        Some("hello")
    );
    assert_eq!(normalize_cached_codex_prompt(Some("   "), false), None);
}

#[test]
fn normalize_cached_codex_prompt_filters_codex_context() {
    let text = " <environment_context>\n  <cwd>/tmp/demo</cwd>\n</environment_context> ";

    assert_eq!(normalize_cached_codex_prompt(Some(text), true), None);
}

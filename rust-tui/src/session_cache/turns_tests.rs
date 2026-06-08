use super::normalize_turns;
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

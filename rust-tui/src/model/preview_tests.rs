use super::{PreviewTurn, SharedPreviewTurns};

#[test]
fn shared_preview_turns_clone_reuses_allocation() {
    let turns = SharedPreviewTurns::from(vec![PreviewTurn {
        question: "hello".into(),
        answer: Some("world".into()),
    }]);
    let cloned = turns.clone();

    assert!(turns.shares_allocation_with(&cloned));
    assert_eq!(cloned[0].question, "hello");
    assert_eq!(cloned[0].answer.as_deref(), Some("world"));
}

#[test]
fn shared_preview_turns_equality_uses_same_allocation() {
    let turns = SharedPreviewTurns::from(vec![PreviewTurn {
        question: "hello".into(),
        answer: Some("world".into()),
    }]);
    let cloned = turns.clone();

    assert_eq!(turns, cloned);
}

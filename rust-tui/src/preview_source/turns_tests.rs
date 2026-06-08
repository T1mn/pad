use super::{finalize_turns, format_session_turns, push_session_message, SessionRole};
use crate::model::PreviewTurn;
use std::collections::VecDeque;

#[test]
fn assistant_messages_append_to_last_question() {
    let mut turns = VecDeque::new();
    push_session_message(&mut turns, SessionRole::User, "question".into());
    push_session_message(&mut turns, SessionRole::Assistant, "line 1".into());
    push_session_message(&mut turns, SessionRole::Assistant, "line 2".into());

    let turns = finalize_turns(turns);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].answer.as_deref(), Some("line 1\nline 2"));
}

#[test]
fn formatting_uses_q_and_a_blocks() {
    let turns = vec![PreviewTurn {
        question: "hello".into(),
        answer: Some("world".into()),
    }];

    assert_eq!(format_session_turns(&turns), "Q:\nhello\n\nA:\nworld");
}

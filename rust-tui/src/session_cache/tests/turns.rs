use super::super::turns::merge_recent_turns;
use crate::model::PreviewTurn;

#[test]
fn merge_recent_turns_prefers_latest_prompt_and_answer() {
    let mut turns = Vec::new();
    merge_recent_turns(&mut turns, Some("hello"), None, None);
    merge_recent_turns(&mut turns, None, Some("world"), Some("hello"));
    assert_eq!(
        turns,
        vec![PreviewTurn {
            question: "hello".to_string(),
            answer: Some("world".to_string()),
        }]
    );
}

#[test]
fn merge_recent_turns_does_not_reuse_previous_answer_for_new_prompt() {
    let mut turns = vec![PreviewTurn {
        question: "old prompt".to_string(),
        answer: Some("old answer".to_string()),
    }];

    merge_recent_turns(&mut turns, Some("new prompt"), None, Some("new prompt"));

    assert_eq!(
        turns,
        vec![
            PreviewTurn {
                question: "new prompt".to_string(),
                answer: None,
            },
            PreviewTurn {
                question: "old prompt".to_string(),
                answer: Some("old answer".to_string()),
            },
        ]
    );
}

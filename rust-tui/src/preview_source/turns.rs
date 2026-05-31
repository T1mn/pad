use crate::model::PreviewTurn;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SessionRole {
    User,
    Assistant,
}

pub(super) fn push_session_message(
    turns: &mut VecDeque<PreviewTurn>,
    role: SessionRole,
    text: String,
) {
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }

    match role {
        SessionRole::User => {
            turns.push_back(PreviewTurn {
                question: text,
                answer: None,
            });
            while turns.len() > crate::session_cache::SESSION_HISTORY_TURN_LIMIT {
                turns.pop_front();
            }
        }
        SessionRole::Assistant => {
            if let Some(last) = turns.back_mut() {
                match last.answer.as_mut() {
                    Some(existing) => {
                        if !existing.is_empty() {
                            existing.push('\n');
                        }
                        existing.push_str(&text);
                    }
                    None => {
                        last.answer = Some(text);
                    }
                }
            }
        }
    }
}

pub(super) fn finalize_turns(turns: VecDeque<PreviewTurn>) -> Vec<PreviewTurn> {
    turns.into_iter().rev().collect()
}

#[cfg(test)]
pub(super) fn format_session_turns(turns: &[PreviewTurn]) -> String {
    turns
        .iter()
        .map(|turn| {
            let answer = turn.answer.as_deref().unwrap_or("...");
            format!("Q:\n{}\n\nA:\n{}", turn.question.trim(), answer.trim())
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
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
}

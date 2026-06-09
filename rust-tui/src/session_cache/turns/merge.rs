use super::normalize::normalize_turns;
use crate::model::PreviewTurn;
use crate::session_cache::util::clean_text;

pub(in crate::session_cache) fn merge_recent_turns(
    turns: &mut Vec<PreviewTurn>,
    prompt: Option<&str>,
    assistant: Option<&str>,
    fallback_question: Option<&str>,
) {
    let prompt = clean_text(prompt);
    let assistant = clean_text(assistant);
    let fallback_question = clean_text(fallback_question);

    if let Some(prompt_text) = prompt.as_deref() {
        insert_prompt_if_needed(turns, prompt_text);
    }

    if let Some(answer_text) = assistant.as_deref() {
        merge_assistant_answer(
            turns,
            prompt.as_deref().or(fallback_question.as_deref()),
            answer_text,
        );
    }

    *turns = normalize_turns(std::mem::take(turns), false);
}

fn insert_prompt_if_needed(turns: &mut Vec<PreviewTurn>, prompt_text: &str) {
    let should_insert = match turns.first() {
        Some(first) => first.question.trim() != prompt_text || first.answer.is_some(),
        None => true,
    };
    if should_insert {
        turns.insert(
            0,
            PreviewTurn {
                question: prompt_text.to_string(),
                answer: None,
            },
        );
    }
}

fn merge_assistant_answer(
    turns: &mut Vec<PreviewTurn>,
    question_hint: Option<&str>,
    answer_text: &str,
) {
    if let Some(first) = turns.first_mut() {
        let question_matches = question_hint
            .map(|hint| first.question.trim() == hint)
            .unwrap_or(true);
        if question_matches || first.answer.is_none() {
            if first.answer.as_deref() != Some(answer_text) {
                first.answer = Some(answer_text.to_string());
            }
        } else if let Some(hint) = question_hint {
            turns.insert(
                0,
                PreviewTurn {
                    question: hint.to_string(),
                    answer: Some(answer_text.to_string()),
                },
            );
        }
    } else if let Some(hint) = question_hint {
        turns.push(PreviewTurn {
            question: hint.to_string(),
            answer: Some(answer_text.to_string()),
        });
    }
}

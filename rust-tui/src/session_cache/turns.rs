mod merge {
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
}
mod normalize {
    use super::super::model::SESSION_HISTORY_TURN_LIMIT;
    use crate::model::PreviewTurn;
    use std::borrow::Borrow;

    pub(in crate::session_cache) fn normalize_turns<I, T>(
        turns: I,
        normalize_codex_prompts: bool,
    ) -> Vec<PreviewTurn>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<PreviewTurn>,
    {
        let mut normalized = Vec::with_capacity(SESSION_HISTORY_TURN_LIMIT);
        for turn in turns {
            if let Some(turn) = normalize_turn(turn.borrow(), normalize_codex_prompts) {
                normalized.push(turn);
                if normalized.len() == SESSION_HISTORY_TURN_LIMIT {
                    break;
                }
            }
        }

        normalized
    }

    fn normalize_turn(turn: &PreviewTurn, normalize_codex_prompts: bool) -> Option<PreviewTurn> {
        let question = if normalize_codex_prompts {
            crate::preview_source::codex::normalize_codex_user_text(&turn.question, None)
        } else {
            turn.question.trim().to_string()
        };
        if question.is_empty() {
            return None;
        }
        let answer = turn
            .answer
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned);
        Some(PreviewTurn { question, answer })
    }
}
mod prompt {
    pub(in crate::session_cache) fn normalize_cached_codex_prompt(
        value: Option<&str>,
        normalize_codex: bool,
    ) -> Option<String> {
        value.and_then(|text| {
            let text = text.trim();
            if text.is_empty() {
                return None;
            }

            let normalized = if normalize_codex {
                crate::preview_source::codex::normalize_codex_user_text_cow(text, None)
            } else {
                std::borrow::Cow::Borrowed(text)
            };
            if normalized.is_empty() {
                None
            } else {
                Some(normalized.into_owned())
            }
        })
    }
}

pub(super) use merge::merge_recent_turns;
pub(super) use normalize::normalize_turns;
pub(super) use prompt::normalize_cached_codex_prompt;

#[cfg(test)]
#[path = "turns_tests.rs"]
pub(crate) mod tests;

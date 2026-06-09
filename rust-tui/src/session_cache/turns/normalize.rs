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

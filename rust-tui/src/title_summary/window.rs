use crate::model::PreviewTurn;

const INITIAL_TURN_THRESHOLD: usize = 3;
const REFRESH_INTERVAL_TURNS: usize = 6;
const INITIAL_WINDOW_TURNS: usize = 3;
const REFRESH_WINDOW_TURNS: usize = 6;

pub fn is_enabled(config: &crate::theme::CodexConfig) -> bool {
    config.title_summary
}

pub fn should_refresh_title(turn_count: usize, generated_turn_count: Option<usize>) -> bool {
    if turn_count < INITIAL_TURN_THRESHOLD {
        return false;
    }

    match generated_turn_count {
        Some(previous) if previous >= INITIAL_TURN_THRESHOLD => {
            turn_count >= previous.saturating_add(REFRESH_INTERVAL_TURNS)
        }
        _ => true,
    }
}

pub fn select_turn_window(
    turns: &[PreviewTurn],
    generated_turn_count: Option<usize>,
) -> Vec<PreviewTurn> {
    let limit = if matches!(generated_turn_count, Some(count) if count >= INITIAL_TURN_THRESHOLD) {
        REFRESH_WINDOW_TURNS
    } else {
        INITIAL_WINDOW_TURNS
    };

    let mut selected = turns
        .iter()
        .filter(|turn| !turn.question.trim().is_empty())
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    selected.reverse();
    selected
}

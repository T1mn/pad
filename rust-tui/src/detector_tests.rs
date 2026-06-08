use super::detect_state;
use crate::model::AgentState;

#[test]
fn busy_keywords_take_priority() {
    assert_eq!(
        detect_state("thinking\n>"),
        AgentState::Busy,
        "busy keyword should win over prompt-looking tail"
    );
}

#[test]
fn waiting_prompt_is_detected_from_last_non_empty_line() {
    assert_eq!(detect_state("done\n\n$"), AgentState::Waiting);
}

#[test]
fn idle_when_no_busy_or_waiting_signal() {
    assert_eq!(detect_state("finished normally"), AgentState::Idle);
}

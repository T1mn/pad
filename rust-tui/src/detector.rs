use crate::scanner::strip_ansi;
use crate::text_match::contains_ascii_ignore_case;

const TAIL_LINES_FOR_STATE: usize = 5;
const BRAILLE_SPINNERS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const STAR_SPINNERS: [char; 4] = ['✳', '✽', '✶', '✢'];
const BUSY_KEYWORDS: [&str; 14] = [
    "thinking",
    "processing",
    "generating",
    "analyzing",
    "compiling",
    "searching",
    "reading",
    "writing",
    "editing",
    "running",
    "ctrl+c to interrupt",
    "esc to interrupt",
    "ctrl-c to interrupt",
    "esc to stop",
];
const WAITING_PATTERNS: [&str; 10] = [
    "yes, allow once",
    "yes, allow always",
    "allow once",
    "continue?",
    "[y/n]",
    "[yes/no]",
    "do you want to",
    "would you like to",
    "permission",
    "approve",
];

/// Detect agent state from the last N lines of pane content.
/// Priority: Busy > Waiting > Idle
pub fn detect_state(raw_content: &str) -> super::model::AgentState {
    use super::model::AgentState;

    let content = strip_ansi(raw_content);

    // --- Busy detection (highest priority) ---
    // Braille spinner characters (U+2800-28FF subset used by CLI spinners)
    if tail_contains_char(&content, &BRAILLE_SPINNERS)
        || tail_contains_char(&content, &STAR_SPINNERS)
        || tail_contains_ascii_pattern(&content, &BUSY_KEYWORDS)
    {
        return AgentState::Busy;
    }

    // --- Waiting detection ---
    if tail_contains_ascii_pattern(&content, &WAITING_PATTERNS) {
        return AgentState::Waiting;
    }
    // Prompt characters at end of last non-empty line
    let last_line = content.lines().rev().find_map(|line| {
        let line = line.trim();
        (!line.is_empty()).then_some(line)
    });
    if let Some(line) = last_line {
        if line.ends_with('>')
            || line.ends_with('❯')
            || line.ends_with('$')
            || line.ends_with('%')
            || line.ends_with('#')
        {
            return AgentState::Waiting;
        }
    }

    AgentState::Idle
}

fn tail_lines_for_state_detection(content: &str) -> impl Iterator<Item = &str> {
    content.lines().rev().take(TAIL_LINES_FOR_STATE)
}

fn tail_contains_char(content: &str, chars: &[char]) -> bool {
    tail_lines_for_state_detection(content).any(|line| chars.iter().any(|ch| line.contains(*ch)))
}

fn tail_contains_ascii_pattern(content: &str, patterns: &[&str]) -> bool {
    tail_lines_for_state_detection(content).any(|line| {
        patterns
            .iter()
            .any(|pattern| contains_ascii_ignore_case(line, pattern))
    })
}

#[cfg(test)]
#[path = "detector_tests.rs"]
mod tests;

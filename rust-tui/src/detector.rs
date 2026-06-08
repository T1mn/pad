use crate::scanner::strip_ansi;

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
    let tail = tail_for_state_detection(&content);
    let tail_lower = tail.to_lowercase();

    // --- Busy detection (highest priority) ---
    // Braille spinner characters (U+2800-28FF subset used by CLI spinners)
    if BRAILLE_SPINNERS.iter().any(|ch| tail.contains(*ch))
        || STAR_SPINNERS.iter().any(|ch| tail.contains(*ch))
        || BUSY_KEYWORDS
            .iter()
            .any(|keyword| tail_lower.contains(keyword))
    {
        return AgentState::Busy;
    }

    // --- Waiting detection ---
    if WAITING_PATTERNS
        .iter()
        .any(|pattern| tail_lower.contains(pattern))
    {
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

fn tail_for_state_detection(content: &str) -> String {
    content
        .lines()
        .rev()
        .take(TAIL_LINES_FOR_STATE)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "detector_tests.rs"]
mod tests;

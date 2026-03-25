use crate::scanner::strip_ansi;

/// Detect agent state from the last N lines of pane content.
/// Priority: Busy > Waiting > Idle
pub fn detect_state(raw_content: &str) -> super::model::AgentState {
    use super::model::AgentState;

    let content = strip_ansi(raw_content);
    // Focus on last 5 lines for prompt/state detection
    let lines: Vec<&str> = content.lines().collect();
    let tail: String = lines
        .iter()
        .rev()
        .take(5)
        .copied()
        .collect::<Vec<&str>>()
        .join("\n");
    let tail_lower = tail.to_lowercase();

    // --- Busy detection (highest priority) ---
    // Braille spinner characters (U+2800-28FF subset used by CLI spinners)
    let braille_spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    for ch in &braille_spinners {
        if tail.contains(*ch) {
            return AgentState::Busy;
        }
    }
    // Asterisk/star spinners
    let star_spinners = ['✳', '✽', '✶', '✢'];
    for ch in &star_spinners {
        if tail.contains(*ch) {
            return AgentState::Busy;
        }
    }
    // Keyword-based busy detection
    let busy_keywords = [
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
    for kw in &busy_keywords {
        if tail_lower.contains(kw) {
            return AgentState::Busy;
        }
    }

    // --- Waiting detection ---
    let waiting_patterns = [
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
    for pat in &waiting_patterns {
        if tail_lower.contains(pat) {
            return AgentState::Waiting;
        }
    }
    // Prompt characters at end of last non-empty line
    let last_line = lines
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim());
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

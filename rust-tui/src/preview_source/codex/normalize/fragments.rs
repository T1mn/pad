const ENVIRONMENT_CONTEXT_OPEN: &str = "<environment_context>";
const ENVIRONMENT_CONTEXT_CLOSE: &str = "</environment_context>";
const TURN_ABORTED_OPEN: &str = "<turn_aborted>";
const TURN_ABORTED_CLOSE: &str = "</turn_aborted>";
const USER_SHELL_COMMAND_OPEN: &str = "<user_shell_command>";
const USER_SHELL_COMMAND_CLOSE: &str = "</user_shell_command>";
const SKILL_OPEN: &str = "<skill>";
const SKILL_CLOSE: &str = "</skill>";
const AGENTS_MD_INSTRUCTIONS_PREFIX: &str = "# AGENTS.md instructions for ";
const AGENTS_MD_INSTRUCTIONS_SUFFIX: &str = "</INSTRUCTIONS>";

pub(super) fn strip_non_preview_codex_fragments(text: &str) -> String {
    let mut stripped = text.to_string();
    for (open, close) in [
        (ENVIRONMENT_CONTEXT_OPEN, ENVIRONMENT_CONTEXT_CLOSE),
        (TURN_ABORTED_OPEN, TURN_ABORTED_CLOSE),
        (USER_SHELL_COMMAND_OPEN, USER_SHELL_COMMAND_CLOSE),
        (SKILL_OPEN, SKILL_CLOSE),
        (AGENTS_MD_INSTRUCTIONS_PREFIX, AGENTS_MD_INSTRUCTIONS_SUFFIX),
    ] {
        stripped = strip_wrapped_block(&stripped, open, close);
    }
    stripped
}

pub(super) fn extract_user_shell_command_summary(text: &str) -> Option<String> {
    let inner = exact_wrapped_fragment(
        text.trim(),
        USER_SHELL_COMMAND_OPEN,
        USER_SHELL_COMMAND_CLOSE,
    )?;
    let command = find_wrapped_fragment(inner.trim(), "<command>", "</command>")
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(format!("[shell] {}", command))
}

fn strip_wrapped_block(text: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find(open) {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + open.len()..];
        let Some(end) = after_open.find(close) else {
            out.push_str(&rest[start..]);
            return out;
        };
        rest = &after_open[end + close.len()..];
    }

    out.push_str(rest);
    out
}

fn exact_wrapped_fragment<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix(open)?.strip_suffix(close)?;
    Some(inner)
}

fn find_wrapped_fragment<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)?;
    let after_open = &text[start + open.len()..];
    let end = after_open.find(close)?;
    Some(&after_open[..end])
}

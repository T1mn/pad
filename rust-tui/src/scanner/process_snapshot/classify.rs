pub(in crate::scanner) fn command_may_hide_agent(command: &str) -> bool {
    is_shell_wrapper(command) || command_args_may_name_agent(command)
}

pub(in crate::scanner) fn command_args_may_name_agent(command: &str) -> bool {
    const ARG_WRAPPERS: &[&str] = &[
        "bun", "deno", "node", "npm", "npx", "pnpm", "python", "python3", "uv", "uvx", "yarn",
    ];
    token_matches(command, ARG_WRAPPERS)
}

fn is_shell_wrapper(command: &str) -> bool {
    const SHELLS: &[&str] = &["bash", "fish", "sh", "zsh"];
    token_matches(command, SHELLS)
}

fn token_matches(command: &str, candidates: &[&str]) -> bool {
    let token = command_token(command);
    candidates
        .iter()
        .any(|candidate| token.eq_ignore_ascii_case(candidate))
}

fn command_token(command: &str) -> &str {
    command
        .split_whitespace()
        .next()
        .and_then(|part| part.rsplit('/').next())
        .unwrap_or(command)
}

#[cfg(test)]
#[path = "classify_tests.rs"]
mod tests;

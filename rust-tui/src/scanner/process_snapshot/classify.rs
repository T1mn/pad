pub(in crate::scanner) fn command_may_hide_agent(command: &str) -> bool {
    is_shell_wrapper(command) || command_args_may_name_agent(command)
}

pub(in crate::scanner) fn command_args_may_name_agent(command: &str) -> bool {
    matches!(
        command_token(command).as_str(),
        "bun"
            | "deno"
            | "node"
            | "npm"
            | "npx"
            | "pnpm"
            | "python"
            | "python3"
            | "uv"
            | "uvx"
            | "yarn"
    )
}

fn is_shell_wrapper(command: &str) -> bool {
    matches!(
        command_token(command).as_str(),
        "bash" | "fish" | "sh" | "zsh"
    )
}

fn command_token(command: &str) -> String {
    command
        .split_whitespace()
        .next()
        .and_then(|part| part.rsplit('/').next())
        .unwrap_or(command)
        .to_ascii_lowercase()
}

#[cfg(test)]
#[path = "classify_tests.rs"]
mod tests;

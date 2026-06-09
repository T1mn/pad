use std::io;

mod auth;
mod command;
#[cfg(test)]
mod tests;

pub use auth::ensure_pad_codex_auth_ready;
pub(crate) use command::shell_single_quote;
pub use command::with_pad_codex_runtime;

pub fn prepare_agent_command(agent_name: &str, agent_cmd: &str) -> io::Result<String> {
    if command::is_claude_agent(agent_name, agent_cmd) {
        return Ok(command::with_pad_claude_runtime(agent_cmd));
    }

    if !command::is_codex_agent(agent_name, agent_cmd) {
        return Ok(agent_cmd.to_string());
    }

    crate::paths::ensure_pad_codex_home_layout()?;
    crate::paths::ensure_pad_codex_wrapper()?;
    ensure_pad_codex_auth_ready()?;
    Ok(with_pad_codex_runtime(agent_cmd))
}

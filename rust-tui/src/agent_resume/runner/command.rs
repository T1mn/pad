use super::shell::{shell_command_name, shell_single_quote};
use crate::agent_resume::model::ResumeTarget;

pub fn build_resume_command(target: &ResumeTarget) -> String {
    let sid = shell_single_quote(&target.agent_session_id);
    let cwd = shell_single_quote(&target.working_dir);
    match target.agent_type.as_str() {
        "codex" => format!(
            "exec {} -C {} resume {}",
            crate::codex_runtime::with_pad_codex_runtime("codex"),
            cwd,
            sid
        ),
        "claude" => format!("exec claude --resume {}", sid),
        "grok" => format!("exec grok --resume {}", sid),
        "gemini" => format!("exec gemini --resume {}", sid),
        "opencode" => format!("exec opencode --session {}", sid),
        other => format!("exec {} --resume {}", shell_command_name(other), sid),
    }
}

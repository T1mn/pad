mod command {
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
}
mod display;
mod execute;
mod plan {
    use super::command::build_resume_command;
    use super::shell::safe_name;
    use crate::agent_resume::model::ResumeTarget;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ResumeLaunchPlan {
        pub tmux_session_name: String,
        pub working_dir: String,
        pub resume_command: String,
        pub tmux_commands: Vec<Vec<String>>,
    }

    pub fn build_launch_plan(target: &ResumeTarget) -> ResumeLaunchPlan {
        let tmux_session_name = format!("pad_resume_{}", safe_name(&target.agent_session_id));
        let resume_command = build_resume_command(target);
        ResumeLaunchPlan {
            tmux_session_name: tmux_session_name.clone(),
            working_dir: target.working_dir.clone(),
            tmux_commands: vec![
                vec![
                    "new-session".into(),
                    "-d".into(),
                    "-s".into(),
                    tmux_session_name.clone(),
                    "-c".into(),
                    target.working_dir.clone(),
                    resume_command.clone(),
                ],
                vec![
                    "switch-client".into(),
                    "-t".into(),
                    tmux_session_name.clone(),
                ],
            ],
            resume_command,
        }
    }
}
mod shell {
    pub(super) fn shell_single_quote(value: &str) -> String {
        crate::shell_quote::single_quote(value)
    }

    pub(super) fn shell_display_quote(value: &str) -> String {
        if value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='))
        {
            value.to_string()
        } else {
            shell_single_quote(value)
        }
    }

    pub(super) fn shell_command_name(value: &str) -> String {
        value
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
            .collect::<String>()
    }

    pub(super) fn safe_name(value: &str) -> String {
        let mut out = String::new();
        for ch in value.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
                out.push(ch);
            } else if !out.ends_with('_') {
                out.push('_');
            }
        }
        out.trim_matches('_').chars().take(40).collect()
    }
}

pub use display::display_tmux_command;
pub use execute::launch_resume_target;

#[cfg(test)]
mod tests;

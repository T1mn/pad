use super::model::ResumeTarget;
use std::io;
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumeLaunchPlan {
    pub tmux_session_name: String,
    pub working_dir: String,
    pub resume_command: String,
    pub tmux_commands: Vec<Vec<String>>,
}

pub fn build_resume_command(target: &ResumeTarget) -> String {
    let sid = shell_single_quote(&target.agent_session_id);
    let cwd = shell_single_quote(&target.working_dir);
    match target.agent_type.as_str() {
        "codex" => format!(
            "exec env CODEX_HOME={} codex -C {} resume {}",
            shell_single_quote(&crate::paths::pad_codex_home_dir().to_string_lossy()),
            cwd,
            sid
        ),
        "claude" => format!("exec claude --resume {}", sid),
        "gemini" => format!("exec gemini --resume {}", sid),
        "opencode" => format!("exec opencode --resume {}", sid),
        other => format!("exec {} --resume {}", shell_command_name(other), sid),
    }
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

pub fn launch_resume_target(target: &ResumeTarget, dry_run: bool) -> io::Result<ResumeLaunchPlan> {
    if target.agent_type == "codex" {
        crate::paths::ensure_pad_codex_home_layout()?;
    }
    let plan = build_launch_plan(target);
    if dry_run {
        return Ok(plan);
    }
    for args in &plan.tmux_commands {
        let output = Command::new("tmux").args(args).output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "tmux {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
    }
    Ok(plan)
}

pub fn display_tmux_command(args: &[String]) -> String {
    std::iter::once("tmux")
        .chain(args.iter().map(String::as_str))
        .map(shell_display_quote)
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

fn shell_display_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='))
    {
        value.to_string()
    } else {
        shell_single_quote(value)
    }
}

fn shell_command_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .collect::<String>()
}

fn safe_name(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_resume_command_uses_pad_codex_home_and_resume() {
        let target = ResumeTarget {
            agent_session_id: "abc 123".into(),
            agent_type: "codex".into(),
            working_dir: "/tmp/demo dir".into(),
            transcript_path: None,
            title: None,
            updated_at: 1,
        };
        let command = build_resume_command(&target);

        assert!(command.contains("CODEX_HOME="));
        assert!(command.contains("codex -C '/tmp/demo dir' resume 'abc 123'"));
    }

    #[test]
    fn launch_plan_wraps_resume_command_in_tmux() {
        let target = ResumeTarget {
            agent_session_id: "sid".into(),
            agent_type: "claude".into(),
            working_dir: "/tmp/demo".into(),
            transcript_path: None,
            title: None,
            updated_at: 1,
        };
        let plan = build_launch_plan(&target);

        assert_eq!(plan.tmux_commands[0][0], "new-session");
        assert!(plan.resume_command.contains("claude --resume 'sid'"));
    }
}

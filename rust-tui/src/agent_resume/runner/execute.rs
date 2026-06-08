use super::plan::{build_launch_plan, ResumeLaunchPlan};
use crate::agent_resume::model::ResumeTarget;
use std::io;
use std::process::Command;

pub fn launch_resume_target(target: &ResumeTarget, dry_run: bool) -> io::Result<ResumeLaunchPlan> {
    if target.agent_type == "codex" {
        crate::paths::ensure_pad_codex_home_layout()?;
        crate::paths::ensure_pad_codex_wrapper()?;
        crate::codex_runtime::ensure_pad_codex_auth_ready()?;
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
                format_tmux_args(args),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
    }
    Ok(plan)
}

fn format_tmux_args(args: &[String]) -> String {
    let mut formatted = String::new();
    for arg in args {
        if !formatted.is_empty() {
            formatted.push(' ');
        }
        formatted.push_str(arg);
    }
    formatted
}

#[cfg(test)]
#[path = "execute_tests.rs"]
mod tests;

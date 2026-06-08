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
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
    }
    Ok(plan)
}

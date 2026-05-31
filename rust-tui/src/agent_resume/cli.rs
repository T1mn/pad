use super::catalog::{find_resume_target, list_resume_targets};
use super::runner::{display_tmux_command, launch_resume_target};
use std::error::Error;

pub fn run_args(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args.into_iter().collect();
    match args.first().map(String::as_str) {
        Some("list") => {
            for target in list_resume_targets() {
                println!("{}", target.label());
            }
            Ok(())
        }
        Some("run") => run_target(args.get(1).map(String::as_str), args.iter()),
        Some("command") => print_command(args.get(1).map(String::as_str)),
        Some(other) => Err(format!("unknown agent-resume command: {other}").into()),
        None => Err(
            "usage: pad __internal agent-resume list|command|run <session-id> [--dry-run]".into(),
        ),
    }
}

fn print_command(session_id: Option<&str>) -> Result<(), Box<dyn Error>> {
    let target = load_target(session_id)?;
    let plan = launch_resume_target(&target, true)?;
    println!("{}", plan.resume_command);
    Ok(())
}

fn run_target<'a>(
    session_id: Option<&str>,
    mut args: impl Iterator<Item = &'a String>,
) -> Result<(), Box<dyn Error>> {
    let dry_run = args.any(|arg| arg == "--dry-run");
    let target = load_target(session_id)?;
    let plan = launch_resume_target(&target, dry_run)?;
    if dry_run {
        for command in &plan.tmux_commands {
            println!("{}", display_tmux_command(command));
        }
    } else {
        println!(
            "resumed {} in tmux session {}",
            target.agent_session_id, plan.tmux_session_name
        );
    }
    Ok(())
}

fn load_target(session_id: Option<&str>) -> Result<super::model::ResumeTarget, Box<dyn Error>> {
    let session_id = session_id.ok_or("missing session id")?;
    find_resume_target(session_id)
        .ok_or_else(|| format!("resume target not found: {session_id}").into())
}

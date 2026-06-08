use super::plan::build_launch_plan;
use super::{RecipeCommand, RecipeRunReport};
use crate::workspace_recipe::display::display_command;
use crate::workspace_recipe::model::WorkspaceRecipe;
use std::io;
use std::process::Command;

pub fn run_recipe(recipe: &WorkspaceRecipe, dry_run: bool) -> io::Result<RecipeRunReport> {
    let plan = build_launch_plan(recipe);
    if dry_run {
        return Ok(RecipeRunReport {
            plan,
            dry_run,
            executed: 0,
        });
    }

    let mut executed = 0;
    for command in &plan.commands {
        run_command(command)?;
        executed += 1;
    }
    for url in &plan.browser_urls {
        if let Err(err) = crate::browser_remote::open_browser_url(url) {
            log_debug!(
                "workspace_recipe: browser open failed url={} err={}",
                url,
                err
            );
        }
    }
    Ok(RecipeRunReport {
        plan,
        dry_run,
        executed,
    })
}

fn run_command(command: &RecipeCommand) -> io::Result<()> {
    let output = Command::new(&command.program)
        .args(&command.args)
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "{} failed: {}",
        display_command(command),
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

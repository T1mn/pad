use super::display::{display_command, shell_quote};
use super::model::{WorkspaceRecipe, WorkspaceRecipeStep};
use std::io;
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecipeLaunchPlan {
    pub recipe_name: String,
    pub session_name: String,
    pub commands: Vec<RecipeCommand>,
    pub browser_urls: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecipeRunReport {
    pub plan: RecipeLaunchPlan,
    pub dry_run: bool,
    pub executed: usize,
}

pub fn build_launch_plan(recipe: &WorkspaceRecipe) -> RecipeLaunchPlan {
    let root = recipe.effective_root();
    let session_name = recipe.effective_session_name();
    let mut plan = RecipeLaunchPlan {
        recipe_name: recipe.name.clone(),
        session_name: session_name.clone(),
        ..Default::default()
    };

    let steps = if recipe.steps.is_empty() {
        vec![WorkspaceRecipeStep {
            name: "shell".into(),
            command: None,
            cwd: None,
            agent: None,
            browser_url: None,
            remote: None,
        }]
    } else {
        recipe.steps.clone()
    };

    for (idx, step) in steps.iter().enumerate() {
        plan.commands
            .push(step_command(&session_name, &root, step, idx == 0));
        if let Some(url) = step
            .browser_url
            .as_ref()
            .filter(|url| !url.trim().is_empty())
        {
            plan.browser_urls.push(url.clone());
        }
    }
    if let Some(url) = recipe
        .browser_url
        .as_ref()
        .filter(|url| !url.trim().is_empty())
    {
        plan.browser_urls.push(url.clone());
    }
    plan.commands.push(RecipeCommand {
        program: "tmux".into(),
        args: vec!["switch-client".into(), "-t".into(), session_name],
    });
    plan
}

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

fn step_command(
    session_name: &str,
    recipe_root: &str,
    step: &WorkspaceRecipeStep,
    first: bool,
) -> RecipeCommand {
    let cwd = step.effective_cwd(recipe_root);
    let command = step_launch_command(step, &cwd);
    if first {
        RecipeCommand {
            program: "tmux".into(),
            args: vec![
                "new-session".into(),
                "-d".into(),
                "-s".into(),
                session_name.to_string(),
                "-n".into(),
                step.name.clone(),
                "-c".into(),
                cwd,
                command,
            ],
        }
    } else {
        RecipeCommand {
            program: "tmux".into(),
            args: vec![
                "new-window".into(),
                "-t".into(),
                session_name.to_string(),
                "-n".into(),
                step.name.clone(),
                "-c".into(),
                cwd,
                command,
            ],
        }
    }
}

fn step_launch_command(step: &WorkspaceRecipeStep, cwd: &str) -> String {
    let command = step.effective_command();
    let Some(host) = step
        .remote
        .as_deref()
        .filter(|host| !host.trim().is_empty())
    else {
        return command;
    };
    let ssh =
        crate::browser_remote::remote_ssh_command(&crate::browser_remote::RemoteCommandRequest {
            host: host.to_string(),
            cwd: Some(cwd.to_string()),
            command,
        });
    format!(
        "{} {} {}",
        ssh[0],
        shell_quote(&ssh[1]),
        shell_quote(&ssh[2])
    )
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

#[cfg(test)]
#[path = "runner_tests.rs"]
mod runner_tests;

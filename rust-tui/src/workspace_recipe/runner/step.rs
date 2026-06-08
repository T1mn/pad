use super::RecipeCommand;
use crate::workspace_recipe::display::shell_quote;
use crate::workspace_recipe::model::WorkspaceRecipeStep;

pub(in crate::workspace_recipe::runner) fn step_command(
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

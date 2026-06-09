use super::step::step_command;
use super::{RecipeCommand, RecipeLaunchPlan};
use crate::workspace_recipe::model::{WorkspaceRecipe, WorkspaceRecipeStep};

pub fn build_launch_plan(recipe: &WorkspaceRecipe) -> RecipeLaunchPlan {
    let root = recipe.effective_root();
    let session_name = recipe.effective_session_name();
    let mut plan = RecipeLaunchPlan {
        recipe_name: recipe.name.clone(),
        session_name: session_name.clone(),
        ..Default::default()
    };

    let default_step;
    let steps = if recipe.steps.is_empty() {
        default_step = WorkspaceRecipeStep {
            name: "shell".into(),
            command: None,
            cwd: None,
            agent: None,
            browser_url: None,
            remote: None,
        };
        std::slice::from_ref(&default_step)
    } else {
        &recipe.steps
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

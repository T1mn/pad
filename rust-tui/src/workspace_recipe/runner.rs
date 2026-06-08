mod execute;
mod plan;
mod step;

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

pub use execute::run_recipe;
pub use plan::build_launch_plan;

#[cfg(test)]
#[path = "runner_tests.rs"]
mod runner_tests;

use super::display::display_command;
use super::model::WorkspaceRecipe;
use super::runner::run_recipe;
use super::storage::{find_recipe, load};
use std::error::Error;

pub fn run_args(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args.into_iter().collect();
    match args.first().map(String::as_str) {
        Some("list") => list_recipes(),
        Some("run") => run_named_recipe(args.get(1).map(String::as_str), args.iter()),
        Some("plan") => plan_named_recipe(args.get(1).map(String::as_str)),
        Some(other) => Err(format!("unknown workspace-recipe command: {other}").into()),
        None => {
            Err("usage: pad __internal workspace-recipe list|plan|run <name> [--dry-run]".into())
        }
    }
}

fn list_recipes() -> Result<(), Box<dyn Error>> {
    let file = load()?;
    for summary in WorkspaceRecipe::summaries(&file.recipes) {
        println!(
            "{}\t{}\t{} steps\t{}",
            summary.name,
            summary.root,
            summary.steps,
            summary.description.unwrap_or_default()
        );
    }
    Ok(())
}

fn plan_named_recipe(name: Option<&str>) -> Result<(), Box<dyn Error>> {
    let recipe = load_recipe(name)?;
    let report = run_recipe(recipe, true)?;
    for command in &report.plan.commands {
        println!("{}", display_command(command));
    }
    for url in &report.plan.browser_urls {
        println!("browser open {}", url);
    }
    Ok(())
}

fn run_named_recipe<'a>(
    name: Option<&str>,
    mut args: impl Iterator<Item = &'a String>,
) -> Result<(), Box<dyn Error>> {
    let dry_run = args.any(|arg| arg == "--dry-run");
    let recipe = load_recipe(name)?;
    let report = run_recipe(recipe, dry_run)?;
    if dry_run {
        for command in &report.plan.commands {
            println!("{}", display_command(command));
        }
    } else {
        println!(
            "launched recipe {} in tmux session {} ({} commands)",
            report.plan.recipe_name, report.plan.session_name, report.executed
        );
    }
    Ok(())
}

fn load_recipe(name: Option<&str>) -> Result<&'static WorkspaceRecipe, Box<dyn Error>> {
    let name = name.ok_or("missing recipe name")?.to_string();
    let file = load()?;
    let recipe = find_recipe(&file.recipes, &name)
        .ok_or_else(|| format!("workspace recipe not found: {name}"))?
        .clone();
    Ok(Box::leak(Box::new(recipe)))
}

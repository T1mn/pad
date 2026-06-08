use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRecipesFile {
    #[serde(default)]
    pub recipes: Vec<WorkspaceRecipe>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRecipe {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub session_name: Option<String>,
    #[serde(default)]
    pub browser_url: Option<String>,
    #[serde(default)]
    pub steps: Vec<WorkspaceRecipeStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRecipeStep {
    pub name: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub browser_url: Option<String>,
    #[serde(default)]
    pub remote: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceRecipeSummary {
    pub name: String,
    pub description: Option<String>,
    pub root: String,
    pub steps: usize,
}

impl WorkspaceRecipe {
    pub fn effective_root(&self) -> String {
        self.root.clone().unwrap_or_else(|| ".".to_string())
    }

    pub fn effective_session_name(&self) -> String {
        self.session_name
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| safe_session_name(&self.name))
    }

    pub fn summaries(recipes: &[WorkspaceRecipe]) -> Vec<WorkspaceRecipeSummary> {
        recipes
            .iter()
            .map(|recipe| WorkspaceRecipeSummary {
                name: recipe.name.clone(),
                description: recipe.description.clone(),
                root: recipe.effective_root(),
                steps: recipe.steps.len(),
            })
            .collect()
    }
}

impl WorkspaceRecipeStep {
    pub fn effective_command(&self) -> String {
        self.command
            .clone()
            .or_else(|| self.agent.as_ref().map(|agent| agent_command(agent)))
            .unwrap_or_else(default_shell_command)
    }

    pub fn effective_cwd(&self, recipe_root: &str) -> String {
        match self.cwd.as_deref() {
            Some(cwd) if Path::new(cwd).is_absolute() => cwd.to_string(),
            Some(cwd) if cwd != "." => PathBuf::from(recipe_root)
                .join(cwd)
                .to_string_lossy()
                .to_string(),
            _ => recipe_root.to_string(),
        }
    }
}

pub fn safe_session_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        "pad_workspace".to_string()
    } else {
        format!("pad_{out}")
    }
}

fn agent_command(agent: &str) -> String {
    match agent.trim().to_lowercase().as_str() {
        "claude" | "claude-code" => "claude".to_string(),
        "codex" => "codex".to_string(),
        "gemini" | "gemini-cli" => "gemini".to_string(),
        "opencode" => "opencode".to_string(),
        "aider" => "aider".to_string(),
        "cursor" => "cursor".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => default_shell_command(),
    }
}

fn default_shell_command() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;

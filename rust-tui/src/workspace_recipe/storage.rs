use super::model::{WorkspaceRecipe, WorkspaceRecipesFile};
use std::fs;
use std::io;
use std::path::Path;

pub fn load() -> io::Result<WorkspaceRecipesFile> {
    load_from_path(&crate::paths::workspace_recipes_path())
}

pub fn load_from_path(path: &Path) -> io::Result<WorkspaceRecipesFile> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(WorkspaceRecipesFile::default())
        }
        Err(err) => return Err(err),
    };
    parse_recipes(&content)
}

pub fn parse_recipes(content: &str) -> io::Result<WorkspaceRecipesFile> {
    toml::from_str::<WorkspaceRecipesFile>(content).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("workspace recipe parse failed: {err}"),
        )
    })
}

pub fn find_recipe<'a>(recipes: &'a [WorkspaceRecipe], name: &str) -> Option<&'a WorkspaceRecipe> {
    recipes.iter().find(|recipe| recipe.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recipe_with_nested_steps() {
        let parsed = parse_recipes(
            r#"
                [[recipes]]
                name = "web"
                root = "/tmp/web"
                browser_url = "http://localhost:3000"

                [[recipes.steps]]
                name = "server"
                command = "npm run dev"

                [[recipes.steps]]
                name = "codex"
                agent = "codex"
            "#,
        )
        .unwrap();

        assert_eq!(parsed.recipes.len(), 1);
        assert_eq!(parsed.recipes[0].steps.len(), 2);
        assert_eq!(parsed.recipes[0].steps[1].effective_command(), "codex");
    }
}

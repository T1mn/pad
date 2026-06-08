use super::parse_recipes;

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

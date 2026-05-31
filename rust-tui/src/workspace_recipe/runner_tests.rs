use super::*;
use crate::workspace_recipe::storage::parse_recipes;

#[test]
fn remote_step_runs_ssh_inside_tmux_window() {
    let file = parse_recipes(
        r#"
                [[recipes]]
                name = "remote"
                root = "/srv/app"

                [[recipes.steps]]
                name = "tests"
                command = "cargo test"
                remote = "devbox"
            "#,
    )
    .unwrap();
    let plan = build_launch_plan(&file.recipes[0]);
    let command = plan.commands[0].args.last().unwrap();

    assert!(command.starts_with("ssh devbox "));
    assert!(command.contains("cd"));
    assert!(command.contains("/srv/app"));
    assert!(command.contains("cargo test"));
}

#[test]
fn plan_uses_new_session_then_new_window() {
    let file = parse_recipes(
        r#"
                [[recipes]]
                name = "demo"
                root = "/tmp/demo"

                [[recipes.steps]]
                name = "server"
                command = "npm run dev"

                [[recipes.steps]]
                name = "codex"
                agent = "codex"
            "#,
    )
    .unwrap();
    let plan = build_launch_plan(&file.recipes[0]);

    assert_eq!(plan.session_name, "pad_demo");
    assert_eq!(plan.commands[0].args[0], "new-session");
    assert_eq!(plan.commands[1].args[0], "new-window");
    assert_eq!(plan.commands[1].args.last().unwrap(), "codex");
}

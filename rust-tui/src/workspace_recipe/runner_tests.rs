use super::plan::build_launch_plan;
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

    // `--` 必须紧跟在 ssh 后面,否则 remote 里的 `-o...` 会被当选项解析。
    assert!(command.starts_with("ssh -- devbox "));
    assert!(command.contains("cd"));
    assert!(command.contains("/srv/app"));
    assert!(command.contains("cargo test"));
}

#[test]
fn remote_step_with_option_shaped_host_fails_instead_of_running_locally() {
    let file = parse_recipes(
        r#"
                [[recipes]]
                name = "remote"
                root = "/srv/app"

                [[recipes.steps]]
                name = "tests"
                command = "cargo test"
                remote = "-oProxyCommand=touch /tmp/pwned"
            "#,
    )
    .unwrap();
    let plan = build_launch_plan(&file.recipes[0]);
    let command = plan.commands[0].args.last().unwrap();

    assert!(
        !command.contains("ssh"),
        "ssh must not be invoked: {command}"
    );
    assert!(
        !command.contains("cargo test"),
        "remote command must not fall back to local execution: {command}"
    );
    assert!(
        command.contains("exit 1"),
        "step must fail loudly: {command}"
    );
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

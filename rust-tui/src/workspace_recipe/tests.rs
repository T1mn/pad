mod display {
    use super::super::display::display_command;
    use crate::workspace_recipe::runner::RecipeCommand;

    #[test]
    fn display_command_quotes_arguments_without_collecting_segments() {
        let command = RecipeCommand {
            program: "tmux".into(),
            args: vec![
                "new-window".into(),
                "-n".into(),
                "agent one".into(),
                "echo ready".into(),
            ],
        };

        assert_eq!(
            display_command(&command),
            "tmux new-window -n 'agent one' 'echo ready'"
        );
    }

    #[test]
    fn display_command_escapes_single_quotes() {
        let command = RecipeCommand {
            program: "tmux".into(),
            args: vec!["display-message".into(), "bob's app".into()],
        };

        assert_eq!(
            display_command(&command),
            r#"tmux display-message 'bob'\''s app'"#
        );
    }
}

mod model {
    use super::super::model::{safe_session_name, WorkspaceRecipeStep};

    #[test]
    fn session_name_is_tmux_safe() {
        assert_eq!(safe_session_name("my app/demo"), "pad_my_app_demo");
        assert_eq!(safe_session_name("///"), "pad_workspace");
    }

    #[test]
    fn relative_step_cwd_is_under_recipe_root() {
        let step = WorkspaceRecipeStep {
            name: "server".into(),
            cwd: Some("frontend".into()),
            command: Some("npm run dev".into()),
            agent: None,
            browser_url: None,
            remote: None,
        };
        assert!(step
            .effective_cwd("/tmp/app")
            .ends_with("/tmp/app/frontend"));
    }

    #[test]
    fn agent_command_matches_known_agents_case_insensitively() {
        let step = WorkspaceRecipeStep {
            name: "agent".into(),
            cwd: None,
            command: None,
            agent: Some("CODEX".into()),
            browser_url: None,
            remote: None,
        };
        assert_eq!(step.effective_command(), "codex");
    }

    #[test]
    fn agent_command_keeps_unknown_agent_lowercase_compatibility() {
        let step = WorkspaceRecipeStep {
            name: "agent".into(),
            cwd: None,
            command: None,
            agent: Some("CUSTOM-CLI".into()),
            browser_url: None,
            remote: None,
        };
        assert_eq!(step.effective_command(), "custom-cli");
    }
}

mod runner {
    use super::super::runner::plan::build_launch_plan;
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
}

mod storage {
    use super::super::storage::parse_recipes;

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

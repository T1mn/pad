use super::{safe_session_name, WorkspaceRecipeStep};

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

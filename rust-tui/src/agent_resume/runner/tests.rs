use super::command::build_resume_command;
use super::plan::build_launch_plan;
use crate::agent_resume::model::ResumeTarget;

#[test]
fn codex_resume_command_uses_pad_profile_and_resume() {
    let target = ResumeTarget {
        agent_session_id: "abc 123".into(),
        agent_type: "codex".into(),
        working_dir: "/tmp/demo dir".into(),
        transcript_path: None,
        title: None,
        updated_at: 1,
    };
    let command = build_resume_command(&target);

    assert!(command.starts_with("exec '"));
    assert!(!command.contains("CODEX_HOME="));
    assert!(command.contains("/.pad/scripts/pad-codex' -C '/tmp/demo dir' resume 'abc 123'"));
}

#[test]
fn opencode_resume_command_uses_session_flag() {
    let target = ResumeTarget {
        agent_session_id: "ses_123".into(),
        agent_type: "opencode".into(),
        working_dir: "/tmp/demo".into(),
        transcript_path: None,
        title: None,
        updated_at: 1,
    };

    assert_eq!(
        build_resume_command(&target),
        "exec opencode --session 'ses_123'"
    );
}

#[test]
fn launch_plan_wraps_resume_command_in_tmux() {
    let target = ResumeTarget {
        agent_session_id: "sid".into(),
        agent_type: "claude".into(),
        working_dir: "/tmp/demo".into(),
        transcript_path: None,
        title: None,
        updated_at: 1,
    };
    let plan = build_launch_plan(&target);

    assert_eq!(plan.tmux_commands[0][0], "new-session");
    assert!(plan.resume_command.contains("claude --resume 'sid'"));
}

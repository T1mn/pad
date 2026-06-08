use super::command::build_resume_command;
use super::shell::safe_name;
use crate::agent_resume::model::ResumeTarget;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumeLaunchPlan {
    pub tmux_session_name: String,
    pub working_dir: String,
    pub resume_command: String,
    pub tmux_commands: Vec<Vec<String>>,
}

pub fn build_launch_plan(target: &ResumeTarget) -> ResumeLaunchPlan {
    let tmux_session_name = format!("pad_resume_{}", safe_name(&target.agent_session_id));
    let resume_command = build_resume_command(target);
    ResumeLaunchPlan {
        tmux_session_name: tmux_session_name.clone(),
        working_dir: target.working_dir.clone(),
        tmux_commands: vec![
            vec![
                "new-session".into(),
                "-d".into(),
                "-s".into(),
                tmux_session_name.clone(),
                "-c".into(),
                target.working_dir.clone(),
                resume_command.clone(),
            ],
            vec![
                "switch-client".into(),
                "-t".into(),
                tmux_session_name.clone(),
            ],
        ],
        resume_command,
    }
}

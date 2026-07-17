use super::resume_target_from_grok_thread;
use crate::grok_history::GrokThreadRef;
use std::path::PathBuf;

#[test]
fn grok_history_thread_becomes_resumable_target() {
    let target = resume_target_from_grok_thread(GrokThreadRef {
        session_id: "grok-session".into(),
        cwd: PathBuf::from("/tmp/project"),
        updated_at: 7,
        transcript_path: PathBuf::from("/tmp/updates.jsonl"),
        title: Some("title".into()),
        model_name: None,
    });
    assert_eq!(target.agent_type, "grok");
    assert_eq!(target.agent_session_id, "grok-session");
    assert_eq!(target.working_dir, "/tmp/project");
}

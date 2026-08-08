mod catalog;
mod cli;
mod model {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ResumeTarget {
        pub agent_session_id: String,
        pub agent_type: String,
        pub working_dir: String,
        pub transcript_path: Option<String>,
        pub title: Option<String>,
        pub updated_at: i64,
    }

    impl ResumeTarget {
        pub fn label(&self) -> String {
            format!(
                "{}\t{}\t{}\t{}",
                self.agent_session_id,
                self.agent_type,
                self.working_dir,
                self.title.clone().unwrap_or_default()
            )
        }
    }
}
mod runner;

pub use catalog::find_resume_target;
pub use cli::run_args;
pub use runner::launch_resume_target;

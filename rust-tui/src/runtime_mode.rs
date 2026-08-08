#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RuntimeMode {
    #[default]
    Native,
    TmuxCompatibility,
}

impl RuntimeMode {
    pub fn from_args(args: &[String]) -> Self {
        if args.iter().any(|argument| argument == "--tmux") {
            Self::TmuxCompatibility
        } else {
            Self::Native
        }
    }

    pub fn uses_tmux(self) -> bool {
        matches!(self, Self::TmuxCompatibility)
    }
}

#[cfg(test)]
#[path = "runtime_mode_tests.rs"]
mod tests;

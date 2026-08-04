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
mod tests {
    use super::*;

    #[test]
    fn native_is_default_and_tmux_requires_an_explicit_flag() {
        assert_eq!(RuntimeMode::from_args(&["pad".into()]), RuntimeMode::Native);
        assert_eq!(
            RuntimeMode::from_args(&["pad".into(), "--native".into()]),
            RuntimeMode::Native
        );
        assert_eq!(
            RuntimeMode::from_args(&["pad".into(), "--tmux".into()]),
            RuntimeMode::TmuxCompatibility
        );
    }
}

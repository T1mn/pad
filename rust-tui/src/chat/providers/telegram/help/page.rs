#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::chat::providers::telegram) enum HelpPage {
    Overview,
    Codex,
    Workflow,
}

impl HelpPage {
    pub(in crate::chat::providers::telegram) fn from_callback(data: &str) -> Option<Self> {
        match data {
            "help:overview" => Some(Self::Overview),
            "help:codex" => Some(Self::Codex),
            "help:workflow" => Some(Self::Workflow),
            _ => None,
        }
    }

    pub(in crate::chat::providers::telegram) fn callback_data(self) -> &'static str {
        match self {
            Self::Overview => "help:overview",
            Self::Codex => "help:codex",
            Self::Workflow => "help:workflow",
        }
    }
}

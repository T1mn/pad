use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityHealth {
    #[default]
    Healthy,
    Lagging,
    Frozen,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityAttemptClassification {
    #[default]
    Normal,
    TransientResumeBootstrap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuityWriteSource {
    Hook,
    Resolver,
}

impl ContinuityWriteSource {
    pub(in crate::session_continuity) fn as_str(self) -> &'static str {
        match self {
            Self::Hook => "hook",
            Self::Resolver => "resolver",
        }
    }
}

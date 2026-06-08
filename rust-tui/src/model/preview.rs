use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewSource {
    Tmux,
    Session,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewSessionOrigin {
    Pane,
    App,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewView {
    Plain,
    SessionList,
    SessionDetail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionCacheState {
    Cached,
    Confirmed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewTurn {
    pub question: String,
    pub answer: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SharedPreviewTurns(Arc<[PreviewTurn]>);

impl SharedPreviewTurns {
    pub fn to_vec(&self) -> Vec<PreviewTurn> {
        self.0.as_ref().to_vec()
    }

    pub fn shares_allocation_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Default for SharedPreviewTurns {
    fn default() -> Self {
        Self(Arc::from([]))
    }
}

impl From<Vec<PreviewTurn>> for SharedPreviewTurns {
    fn from(turns: Vec<PreviewTurn>) -> Self {
        Self(turns.into())
    }
}

impl Deref for SharedPreviewTurns {
    type Target = [PreviewTurn];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl AsRef<[PreviewTurn]> for SharedPreviewTurns {
    fn as_ref(&self) -> &[PreviewTurn] {
        self.0.as_ref()
    }
}

impl PartialEq for SharedPreviewTurns {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.as_ref() == other.0.as_ref()
    }
}

impl Eq for SharedPreviewTurns {}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;

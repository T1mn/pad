use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PendingSidebarSpaceActionKind {
    ToggleFolder(String),
    CollapseParentFolder(String),
}

#[derive(Clone, Debug)]
pub(crate) struct PendingSidebarSpaceAction {
    pub kind: PendingSidebarSpaceActionKind,
    pub deadline: Instant,
}

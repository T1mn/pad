/// Application mode
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Settings,
    ThemeSelector,
    Tree,
    TreeSearch,
    AgentLauncher,
    DeleteConfirm,
    Help,
    FuzzyPicker,
    RelaySettings,
    FilePreview,
}

/// Relay settings sub-view
#[derive(Clone, Copy, PartialEq)]
pub enum RelayView {
    AgentList,
    ProviderList,
    DetailPane,
}

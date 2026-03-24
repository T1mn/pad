/// Application mode
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Settings,
    ThemeSelector,
    LanguageSelector,
    Tree,
    TreeSearch,
    AgentLauncher,
    DeleteConfirm,
    Help,
    FuzzyPicker,
    RelaySettings,
    FilePreview,
    AgentStyleSettings,
}

/// Relay settings sub-view
#[derive(Clone, Copy, PartialEq)]
pub enum RelayView {
    AgentList,
    ProviderList,
    DetailPane,
}

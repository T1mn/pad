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
    ThreadActionConfirm,
    Help,
    FuzzyPicker,
    RelaySettings,
    FilePreview,
    AgentStyleSettings,
    TelegramSettings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Panel,
    Preview,
}

/// Relay settings sub-view
#[derive(Clone, Copy, PartialEq)]
pub enum RelayView {
    AgentList,
    ProviderList,
    DetailPane,
}

pub mod preview;
pub mod sidebar;

/// Application mode
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Settings,
    #[allow(dead_code)]
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocus {
    List,
    Detail,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsDetailKind {
    Theme,
    AutoRefresh,
    CodexSettings,
    ClaudeFullAccess,
    Relay,
    Telegram,
    AgentStyle,
    PreviewMode,
    DisplayMode,
    Trash,
    Language,
    Version,
}

/// Relay settings sub-view
#[derive(Clone, Copy, PartialEq)]
pub enum RelayView {
    AgentList,
    ProviderList,
    DetailPane,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RelayPopupMode {
    None,
    OpenCodeModels,
    OpenCodeDefaultModel,
    OpenCodeSmallModel,
}

pub use preview::{
    CopyToast, PreviewDetailCache, PreviewDetailRenderRequest, PreviewMouseSelection,
    PreviewPlainCache, PreviewState, ThreadPreviewCacheEntry,
};
pub use sidebar::{
    PendingThreadAction, SidebarState, ThreadActionKind, ThreadListView, ThreadMetaEditKind,
};

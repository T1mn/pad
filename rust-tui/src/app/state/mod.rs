pub mod preview;
pub mod sidebar;

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocus {
    List,
    Detail,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsDetailKind {
    Theme,
    AutoRefresh,
    Relay,
    Telegram,
    AgentStyle,
    PreviewMode,
    DisplayMode,
    Language,
    RefreshInterval,
    Version,
}

/// Relay settings sub-view
#[derive(Clone, Copy, PartialEq)]
pub enum RelayView {
    AgentList,
    ProviderList,
    DetailPane,
}

pub use preview::{
    CopyToast, PreviewDetailCache, PreviewDetailRenderRequest, PreviewMouseSelection,
    PreviewPlainCache, PreviewState, ThreadPreviewCacheEntry,
};
pub use sidebar::{PendingThreadAction, SidebarState, ThreadActionKind, ThreadMetaEditKind};

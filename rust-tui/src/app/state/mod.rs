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
    NotificationInbox,
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
    Sound,
    Relay,
    Telegram,
    AgentStyle,
    PreviewMode,
    DisplayMode,
    Trash,
    Language,
    Version,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodexSettingsView {
    Categories,
    Runtime,
    StatusLine,
    Prompts,
    Preview,
    Cli,
}

impl CodexSettingsView {
    pub const CATEGORY_COUNT: usize = 5;

    pub fn from_category_index(index: usize) -> Self {
        match index {
            0 => Self::Runtime,
            1 => Self::StatusLine,
            2 => Self::Prompts,
            3 => Self::Preview,
            _ => Self::Cli,
        }
    }

    pub fn category_index(self) -> usize {
        match self {
            Self::Categories | Self::Runtime => 0,
            Self::StatusLine => 1,
            Self::Prompts => 2,
            Self::Preview => 3,
            Self::Cli => 4,
        }
    }

    pub fn item_count(self) -> usize {
        match self {
            Self::Categories => Self::CATEGORY_COUNT,
            Self::Runtime => 5,
            Self::StatusLine => 4,
            Self::Prompts => 2,
            Self::Preview => 2,
            Self::Cli => 1,
        }
    }
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
    PreviewPlainCache, PreviewSessionListCache, PreviewSessionListItemCache, PreviewState,
    ThreadPreviewCacheEntry,
};
pub use sidebar::{
    PendingThreadAction, PreferredPanelWidthCache, SidebarState, ThreadActionKind, ThreadListView,
    ThreadMetaEditKind, VisibleSidebarStats,
};

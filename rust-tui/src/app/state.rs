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
}

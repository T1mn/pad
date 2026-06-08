mod apply;
mod remove;
mod state;

pub(super) use apply::apply_codex_runtime_overlay;
pub(super) use remove::remove_codex_runtime_overlay;

pub(super) struct CodexRuntimeOverlay<'a> {
    pub(super) yolo_enabled: bool,
    pub(super) fast_enabled: bool,
    pub(super) goals_enabled: bool,
    pub(super) multi_agent_enabled: bool,
    pub(super) web_search_mode: &'a str,
    pub(super) status_line_items: &'a [&'a str],
    pub(super) jailbreak_prompt_file_enabled: bool,
    pub(super) index_prompt_file_enabled: bool,
}

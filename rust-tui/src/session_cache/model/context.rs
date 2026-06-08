use crate::hook::HookEvent;

#[derive(Default)]
pub(in crate::session_cache) struct HookBindingContext {
    pub session_name: Option<String>,
    pub window_index: Option<String>,
    pub pane_index: Option<String>,
    pub path: Option<String>,
}

impl HookBindingContext {
    pub(in crate::session_cache) fn from_event(event: &HookEvent) -> Self {
        Self {
            session_name: event.tmux.session_name.clone(),
            window_index: event.tmux.window_index.clone(),
            pane_index: event.tmux.pane_index.clone(),
            path: event
                .tmux
                .pane_current_path
                .clone()
                .or_else(|| event.cwd.clone()),
        }
    }
}

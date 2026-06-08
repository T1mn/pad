use crate::app::App;
use crate::log_debug;
use crate::tmux_bindings::{current_root_binding, PAD_SIDER_TOGGLE_KEYS};

use super::super::super::tmux::summarize_log_text;

pub(super) struct SavedBindings {
    pub(super) f12: Option<String>,
    pub(super) cq: Option<String>,
    pub(super) sider: Vec<(&'static str, Option<String>)>,
}

impl SavedBindings {
    pub(super) fn capture_into_app(app: &mut App) -> Self {
        let f12 = current_root_binding("F12");
        let cq = current_root_binding("C-q");
        let sider = PAD_SIDER_TOGGLE_KEYS
            .iter()
            .map(|key| (*key, current_root_binding(key)))
            .collect::<Vec<_>>();

        app.saved_tmux_bindings.clear();
        if let Some(line) = &f12 {
            app.saved_tmux_bindings.push(line.clone());
        }
        if let Some(line) = &cq {
            app.saved_tmux_bindings.push(line.clone());
        }
        for (_, saved_binding) in &sider {
            if let Some(line) = saved_binding {
                app.saved_tmux_bindings.push(line.clone());
            }
        }

        log_debug!(
            "install_return_bindings: saved_bindings f12={} cq={}",
            f12.as_deref()
                .map(summarize_log_text)
                .unwrap_or_else(|| "-".to_string()),
            cq.as_deref()
                .map(summarize_log_text)
                .unwrap_or_else(|| "-".to_string())
        );

        Self { f12, cq, sider }
    }
}

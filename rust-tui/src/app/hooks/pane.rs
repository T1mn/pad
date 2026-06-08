mod effects;
mod panel_update;
mod subagent;

use super::App;
use crate::hook::HookEvent;

impl App {
    pub(super) fn apply_pane_hook_event(&mut self, event: HookEvent, pane_id: String) {
        let panel_item_focused = self.panel_item_is_focused(&pane_id);
        let should_refresh_preview = self
            .selected_panel()
            .map(|panel| panel.pane_id == pane_id)
            .unwrap_or(false);

        let mut pending_effects = None;

        if let Some(panel) = self.panels.iter_mut().find(|p| p.pane_id == pane_id) {
            if subagent::handle_codex_subagent_event(panel, &event, &pane_id) {
                self.invalidate_sidebar_cache();
                if should_refresh_preview {
                    self.invalidate_preview();
                }
                self.dirty = true;
                return;
            }

            let persisted_snapshot =
                panel_update::apply_panel_hook_event(panel, &event, panel_item_focused);
            pending_effects = Some(effects::PendingPaneHookEffects::from_panel(
                &self.config.codex,
                panel,
                &event,
                persisted_snapshot.as_ref(),
            ));

            self.invalidate_sidebar_cache();
            if should_refresh_preview {
                self.invalidate_preview();
            }
            self.dirty = true;
        }

        if let Some(effects) = pending_effects {
            effects.apply(self);
        }
    }
}

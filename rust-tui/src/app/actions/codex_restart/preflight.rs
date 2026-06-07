use super::messages::is_cjk_locale;
use crate::i18n::Locale;
use crate::model::AgentType;

pub(super) fn codex_restart_preflight_message(
    panel: &crate::model::AgentPanel,
    locale: Locale,
) -> Option<&'static str> {
    if panel.agent_type != AgentType::Codex {
        Some(codex_only_message(locale))
    } else {
        None
    }
}

pub(super) fn no_panel_message(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "没有选中的面板"
    } else {
        "No selected panel"
    }
}

fn codex_only_message(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "只支持 Codex 面板"
    } else {
        "Only Codex panels can be restarted"
    }
}

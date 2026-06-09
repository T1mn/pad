use super::super::helpers::localized;
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
    localized(locale, "没有选中的面板", "No selected panel")
}

fn codex_only_message(locale: Locale) -> &'static str {
    localized(
        locale,
        "只支持 Codex 面板",
        "Only Codex panels can be restarted",
    )
}

use super::pane::pane_exists;
use super::run_tmux;

const TARGET_OPTION: &str = "@pad_sider_pane";
const HELPER_TARGET_OPTION: &str = "@pad_sider_target";
const TARGET_WAS_ZOOMED_OPTION: &str = "@pad_sider_target_was_zoomed";

pub(super) fn resolve_target_pane(invoked_pane: &str) -> String {
    if let Some(target_pane) = show_option(invoked_pane, HELPER_TARGET_OPTION)
        .filter(|value| !value.trim().is_empty())
        .filter(|pane| pane_exists(pane))
    {
        return target_pane;
    }
    invoked_pane.to_string()
}

pub(super) fn helper_pane_for_target(target_pane: &str) -> Result<Option<String>, String> {
    let helper = show_option(target_pane, TARGET_OPTION).filter(|value| !value.trim().is_empty());
    match helper {
        Some(helper_pane) if pane_exists(&helper_pane) => Ok(Some(helper_pane)),
        Some(_) => {
            unset_option(target_pane, TARGET_OPTION)?;
            Ok(None)
        }
        None => Ok(None),
    }
}

pub(super) fn remember_target_zoom(target_pane: &str, zoomed: bool) -> Result<(), String> {
    set_option(
        target_pane,
        TARGET_WAS_ZOOMED_OPTION,
        if zoomed { "1" } else { "0" },
    )
}

pub(super) fn should_restore_target_zoom(value: Option<String>) -> bool {
    value.as_deref() == Some("1")
}

pub(super) fn target_was_zoomed(target_pane: &str) -> Option<String> {
    show_option(target_pane, TARGET_WAS_ZOOMED_OPTION)
}

pub(super) fn clear_target_was_zoomed(target_pane: &str) -> Result<(), String> {
    unset_option(target_pane, TARGET_WAS_ZOOMED_OPTION)
}

pub(super) fn remember_helper_for_target(
    target_pane: &str,
    helper_pane: &str,
) -> Result<(), String> {
    set_option(target_pane, TARGET_OPTION, helper_pane)
}

pub(super) fn remember_target_for_helper(
    helper_pane: &str,
    target_pane: &str,
) -> Result<(), String> {
    set_option(helper_pane, HELPER_TARGET_OPTION, target_pane)
}

fn show_option(target: &str, key: &str) -> Option<String> {
    run_tmux(&["show-options", "-p", "-v", "-t", target, key])
        .ok()
        .map(|value| value.trim().to_string())
}

fn set_option(target: &str, key: &str, value: &str) -> Result<(), String> {
    run_tmux(&["set-option", "-p", "-t", target, key, value]).map(|_| ())
}

fn unset_option(target: &str, key: &str) -> Result<(), String> {
    run_tmux(&["set-option", "-p", "-t", target, "-u", key]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::should_restore_target_zoom;

    #[test]
    fn zoom_restore_option_only_restores_explicit_zoomed_targets() {
        assert!(should_restore_target_zoom(Some("1".into())));
        assert!(!should_restore_target_zoom(Some("0".into())));
        assert!(!should_restore_target_zoom(None));
    }
}

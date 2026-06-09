use super::super::helpers::localized;
use crate::i18n::Locale;

pub(super) fn github_started_title(locale: Locale) -> &'static str {
    localized(
        locale,
        "OpenCode GitHub install 已启动",
        "OpenCode GitHub Install Started",
    )
}

pub(super) fn github_failed_title(locale: Locale) -> &'static str {
    localized(
        locale,
        "OpenCode GitHub install 失败",
        "OpenCode GitHub Install Failed",
    )
}

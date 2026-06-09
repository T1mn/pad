use super::super::helpers::localized;
use crate::i18n::Locale;

pub(super) fn import_saved_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode 已导入", "OpenCode Imported")
}

pub(super) fn import_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode 导入失败", "OpenCode Import Failed")
}

use crate::i18n::Locale;

pub(super) fn import_saved_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 已导入"
    } else {
        "OpenCode Imported"
    }
}

pub(super) fn import_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 导入失败"
    } else {
        "OpenCode Import Failed"
    }
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

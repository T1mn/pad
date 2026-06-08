use super::super::super::common::is_cjk_locale;
use crate::app::ThreadMetaEditKind;
use crate::i18n::Locale;

pub(in crate::ui::modals::dialogs) fn thread_meta_editor_title(
    locale: Locale,
    kind: ThreadMetaEditKind,
) -> &'static str {
    match (is_cjk_locale(locale), kind) {
        (true, ThreadMetaEditKind::Title) => "编辑标题",
        (true, ThreadMetaEditKind::Tags) => "编辑标签",
        (false, ThreadMetaEditKind::Title) => "Edit Title",
        (false, ThreadMetaEditKind::Tags) => "Edit Tags",
    }
}

pub(in crate::ui::modals::dialogs) fn thread_meta_editor_field_label(
    locale: Locale,
    kind: ThreadMetaEditKind,
) -> &'static str {
    match (is_cjk_locale(locale), kind) {
        (true, ThreadMetaEditKind::Title) => "标题:",
        (true, ThreadMetaEditKind::Tags) => "标签（逗号分隔）:",
        (false, ThreadMetaEditKind::Title) => "Title:",
        (false, ThreadMetaEditKind::Tags) => "Tags (comma separated):",
    }
}

pub(in crate::ui::modals::dialogs) fn thread_meta_editor_prompt_text(
    locale: Locale,
    kind: ThreadMetaEditKind,
) -> &'static str {
    match (is_cjk_locale(locale), kind) {
        (true, ThreadMetaEditKind::Title) => "输入新的标题",
        (true, ThreadMetaEditKind::Tags) => "输入标签，多个标签用逗号分隔",
        (false, ThreadMetaEditKind::Title) => "Type a new title",
        (false, ThreadMetaEditKind::Tags) => "Type tags separated by commas",
    }
}

pub(in crate::ui::modals::dialogs) fn thread_meta_editor_help_text(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "Enter 保存 | Shift+Delete 清空 | Esc 取消 | 支持粘贴"
    } else {
        "Enter save | Shift+Delete clear | Esc cancel | Paste supported"
    }
}

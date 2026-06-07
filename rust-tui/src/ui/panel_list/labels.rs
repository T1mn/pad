use crate::app::state::ThreadListView;
use crate::i18n::Locale;

pub(super) fn special_view_title_label(locale: Locale, view: ThreadListView) -> &'static str {
    match view {
        ThreadListView::Archived => {
            if is_cjk_locale(locale) {
                "归档"
            } else {
                "Archived"
            }
        }
        ThreadListView::Trash => {
            if is_cjk_locale(locale) {
                "回收站"
            } else {
                "Trash"
            }
        }
        ThreadListView::Normal => "",
    }
}

pub(super) fn display_scope_title_label(locale: Locale, live_only: bool) -> &'static str {
    if is_cjk_locale(locale) {
        if live_only {
            "在线"
        } else {
            "全部"
        }
    } else if live_only {
        "LIVE"
    } else {
        "ALL"
    }
}

pub(super) fn special_view_empty_title(locale: Locale, view: ThreadListView) -> &'static str {
    match view {
        ThreadListView::Archived => {
            if is_cjk_locale(locale) {
                "没有归档会话"
            } else {
                "No archived threads"
            }
        }
        ThreadListView::Trash => {
            if is_cjk_locale(locale) {
                "回收站为空"
            } else {
                "Trash is empty"
            }
        }
        ThreadListView::Normal => "",
    }
}

pub(super) fn special_view_empty_hint(locale: Locale, view: ThreadListView) -> &'static str {
    match view {
        ThreadListView::Archived => {
            if is_cjk_locale(locale) {
                "当前没有可恢复的归档会话"
            } else {
                "There are no archived threads to restore"
            }
        }
        ThreadListView::Trash => {
            if is_cjk_locale(locale) {
                "还没有被 d 隐藏的线程"
            } else {
                "No threads have been hidden with d yet"
            }
        }
        ThreadListView::Normal => "",
    }
}

pub(super) fn special_view_empty_back_hint(locale: Locale, view: ThreadListView) -> &'static str {
    match view {
        ThreadListView::Archived => {
            if is_cjk_locale(locale) {
                "按 'Z' 返回普通视图"
            } else {
                "Press 'Z' to return to the main view"
            }
        }
        ThreadListView::Trash => {
            if is_cjk_locale(locale) {
                "从设置重新进入或按 Esc 退出特殊视图"
            } else {
                "Re-open from Settings or press Esc to leave the special view"
            }
        }
        ThreadListView::Normal => "",
    }
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

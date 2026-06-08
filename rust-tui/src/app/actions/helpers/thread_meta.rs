use super::*;

pub(in crate::app::actions) fn parse_thread_tags(input: &str) -> Vec<String> {
    input
        .split([',', '\n', ';'])
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_string())
        .collect()
}

pub(in crate::app::actions) fn thread_meta_save_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "保存失败"
    } else {
        "Save failed"
    }
}

pub(in crate::app::actions) fn thread_meta_toast(
    locale: Locale,
    kind: ThreadMetaEditKind,
    input: &str,
) -> (&'static str, String) {
    let empty_title = is_cjk_locale(locale);
    match kind {
        ThreadMetaEditKind::Title => {
            if input.is_empty() {
                if empty_title {
                    ("标题已清空", String::from("将回退到上游标题"))
                } else {
                    (
                        "Title cleared",
                        String::from("Will fall back to upstream title"),
                    )
                }
            } else if empty_title {
                ("标题已保存", input.to_string())
            } else {
                ("Title saved", input.to_string())
            }
        }
        ThreadMetaEditKind::Tags => {
            if input.is_empty() {
                if empty_title {
                    ("标签已清空", String::from("无标签"))
                } else {
                    ("Tags cleared", String::from("No tags"))
                }
            } else if empty_title {
                ("标签已保存", input.to_string())
            } else {
                ("Tags saved", input.to_string())
            }
        }
    }
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

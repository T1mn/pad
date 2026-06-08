use crate::i18n::Locale;

#[test]
fn settings_selection_keyword_includes_english_aliases() {
    let keyword = crate::app::actions::settings_item_search_blob(
        Locale::ZhCN,
        "relay",
        "配置",
        "settings.relay",
        "settings.relay",
    );
    assert!(keyword.contains("relay"));
    assert!(keyword.contains("provider"));
}

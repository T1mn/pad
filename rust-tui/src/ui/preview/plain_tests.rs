use super::ensure_plain_preview_cache;
use crate::app::App;

#[test]
fn ensure_plain_preview_cache_reuses_existing_cache_when_context_is_unchanged() {
    let mut app = App::new();
    app.preview.pane_id = Some("%1".into());
    app.preview.content = "plain text".into();

    ensure_plain_preview_cache(&mut app, 12);
    let initial_cache = app.preview.plain_cache.clone().expect("cache built");

    ensure_plain_preview_cache(&mut app, 12);
    let repeated_cache = app.preview.plain_cache.expect("cache reused");

    assert_eq!(initial_cache.target_key, repeated_cache.target_key);
    assert_eq!(initial_cache.width, repeated_cache.width);
    assert_eq!(initial_cache.theme_name, repeated_cache.theme_name);
    assert_eq!(initial_cache.wrapped_rows, repeated_cache.wrapped_rows);
    assert_eq!(initial_cache.lines.len(), repeated_cache.lines.len());
}

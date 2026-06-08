use crate::app::{App, PreviewSessionListCache};

pub(super) fn session_list_cache_matches(
    cache: &PreviewSessionListCache,
    app: &App,
    target_key: &str,
    width: u16,
    theme_name: &str,
) -> bool {
    cache.target_key == target_key
        && cache.width == width
        && cache.theme_name == theme_name
        && cache.items.len() == app.preview.turns.len()
        && (cache.turns.shares_allocation_with(&app.preview.turns)
            || cache.turns.as_ref() == app.preview.turns.as_ref())
}

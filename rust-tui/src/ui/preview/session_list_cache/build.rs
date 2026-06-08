use super::matchers::session_list_cache_matches;
use crate::app::{App, PreviewSessionListCache, PreviewSessionListItemCache};
use crate::theme::Theme;
use crate::ui::preview::session::render_session_card;

pub(crate) fn ensure_session_list_cache(app: &mut App, width: u16, theme: &Theme) {
    let target_key = app.preview.pane_id.as_deref().unwrap_or_default();
    let theme_name = theme.name;
    let cache_hit = app
        .preview
        .session_list_cache
        .as_ref()
        .is_some_and(|cache| session_list_cache_matches(cache, app, target_key, width, theme_name));
    if cache_hit {
        return;
    }

    let started_at = std::time::Instant::now();
    let previous = app.preview.session_list_cache.take();
    let previous_cache = previous
        .filter(|cache| {
            cache.target_key == target_key && cache.width == width && cache.theme_name == theme_name
        })
        .map(|cache| (cache.turns, cache.items));
    let (previous_turns, previous_items) =
        previous_cache.unwrap_or_else(|| (Default::default(), Vec::new()));
    let render_width = width.max(8) as usize;
    let mut reused = 0usize;
    let items = app
        .preview
        .turns
        .iter()
        .enumerate()
        .map(|(index, turn)| {
            if let Some(cached) = previous_items
                .get(index)
                .filter(|_| previous_turns.get(index) == Some(turn))
            {
                reused += 1;
                return cached.clone();
            }
            PreviewSessionListItemCache {
                normal_lines: render_session_card(turn, false, render_width, theme),
                selected_lines: render_session_card(turn, true, render_width, theme),
            }
        })
        .collect::<Vec<_>>();
    log_slow_cache_rebuild(target_key, width, items.len(), reused, started_at.elapsed());
    app.preview.session_list_cache = Some(PreviewSessionListCache {
        target_key: target_key.to_string(),
        width,
        theme_name: theme_name.to_string(),
        turns: app.preview.turns.clone(),
        items,
    });
}

fn log_slow_cache_rebuild(
    target_key: &str,
    width: u16,
    turn_count: usize,
    reused: usize,
    elapsed: std::time::Duration,
) {
    if elapsed >= std::time::Duration::from_millis(8) {
        crate::log_debug!(
            "preview.session_list: cache_rebuild target={} width={} turns={} reused={} elapsed_ms={}",
            target_key,
            width,
            turn_count,
            reused,
            elapsed.as_millis()
        );
    }
}

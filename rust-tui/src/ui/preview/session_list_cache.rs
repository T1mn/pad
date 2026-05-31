use super::session::{
    render_session_card, render_session_gap_line, session_list_total_lines,
    SESSION_ITEM_CONTENT_HEIGHT, SESSION_ITEM_GAP_HEIGHT,
};
use crate::app::{App, PreviewSessionListCache, PreviewSessionListItemCache};
use crate::theme::Theme;
use ratatui::text::Line;

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
    let elapsed = started_at.elapsed();
    if elapsed >= std::time::Duration::from_millis(8) {
        crate::log_debug!(
            "preview.session_list: cache_rebuild target={} width={} turns={} reused={} elapsed_ms={}",
            target_key,
            width,
            items.len(),
            reused,
            elapsed.as_millis()
        );
    }
    app.preview.session_list_cache = Some(PreviewSessionListCache {
        target_key: target_key.to_string(),
        width,
        theme_name: theme_name.to_string(),
        turns: app.preview.turns.clone(),
        items,
    });
}

fn session_list_cache_matches(
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

pub(crate) fn selected_session_list_range(
    selected_turn: Option<usize>,
    turn_count: usize,
) -> Option<(usize, usize)> {
    let selected = selected_turn.filter(|index| *index < turn_count)?;
    let stride = SESSION_ITEM_CONTENT_HEIGHT + SESSION_ITEM_GAP_HEIGHT;
    let start = selected * stride;
    Some((start, start + SESSION_ITEM_CONTENT_HEIGHT - 1))
}

pub(crate) fn visible_session_list_lines(
    app: &App,
    width: usize,
    theme: &Theme,
    scroll: u16,
    height: u16,
) -> Vec<Line<'static>> {
    let Some(cache) = app.preview.session_list_cache.as_ref() else {
        return Vec::new();
    };
    let start = scroll as usize;
    let end = start.saturating_add(height as usize);
    (start..end)
        .filter_map(|line_index| {
            session_list_cached_line(cache, app.preview.selected_turn, line_index, width, theme)
        })
        .collect()
}

fn session_list_cached_line(
    cache: &PreviewSessionListCache,
    selected_turn: Option<usize>,
    line_index: usize,
    width: usize,
    theme: &Theme,
) -> Option<Line<'static>> {
    if line_index >= session_list_total_lines(cache.items.len()) {
        return None;
    }
    let stride = SESSION_ITEM_CONTENT_HEIGHT + SESSION_ITEM_GAP_HEIGHT;
    let turn_index = line_index / stride;
    let offset = line_index % stride;
    if offset >= SESSION_ITEM_CONTENT_HEIGHT {
        return Some(render_session_gap_line(width, theme));
    }
    let item = cache.items.get(turn_index)?;
    let lines = if selected_turn == Some(turn_index) {
        &item.selected_lines
    } else {
        &item.normal_lines
    };
    lines.get(offset).cloned()
}

#[cfg(test)]
mod tests {
    use super::ensure_session_list_cache;
    use crate::app::App;
    use crate::model::{PreviewTurn, SharedPreviewTurns};
    use crate::theme::Theme;

    #[test]
    fn cache_keeps_turn_allocation_for_fast_hits() {
        let turns = SharedPreviewTurns::from(vec![PreviewTurn {
            question: "question".into(),
            answer: Some("answer".into()),
        }]);
        let mut app = App::new();
        app.preview.pane_id = Some("%1".into());
        app.preview.turns = turns.clone();

        ensure_session_list_cache(&mut app, 80, &Theme::default());

        let cache = app.preview.session_list_cache.as_ref().expect("cache");
        assert!(cache.turns.shares_allocation_with(&turns));
        assert_eq!(cache.items.len(), 1);
    }
}

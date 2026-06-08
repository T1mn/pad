use crate::app::App;
use crate::model::{PreviewSource, PreviewView};
use crate::preview_source::PreviewUpdate;

pub(super) fn apply_preview_state(app: &mut App, update: &PreviewUpdate, content: String) {
    let should_follow_bottom = app.preview.follow_bottom
        || app.preview.pane_id.is_none()
        || app.preview.pane_id.as_deref() != Some(update.target_key.as_str());
    let same_context = app.preview.pane_id.as_deref() == Some(update.target_key.as_str())
        && app.preview.source == update.source
        && app.preview.session_origin == update.session_origin
        && app.preview.session_id == update.session_id;

    app.preview.content = content;
    app.preview.pane_id = Some(update.target_key.clone());
    app.preview.source = update.source;
    app.preview.session_origin = update.session_origin;
    app.preview.session_id = update.session_id.clone();

    if app.preview.source == PreviewSource::Session && !update.turns.is_empty() {
        apply_session_preview_state(app, update, same_context);
    } else {
        apply_plain_preview_state(app, should_follow_bottom);
    }
}

fn apply_session_preview_state(app: &mut App, update: &PreviewUpdate, same_context: bool) {
    if !same_context {
        app.preview.selected_turn = None;
        app.preview.expanded_turn = None;
        app.preview.view = PreviewView::SessionList;
        app.preview.detail_scroll = 0;
        app.preview.list_scroll = 0;
        app.preview.follow_selection = true;
    } else {
        app.preview.selected_turn = app
            .preview
            .selected_turn
            .filter(|idx| *idx < update.turns.len());
        app.preview.expanded_turn = app
            .preview
            .expanded_turn
            .filter(|idx| *idx < update.turns.len());
        app.preview.view = if app.preview.expanded_turn.is_some() {
            PreviewView::SessionDetail
        } else {
            PreviewView::SessionList
        };
    }
    app.preview.turns = update.turns.clone();
    app.preview.follow_bottom = false;
}

fn apply_plain_preview_state(app: &mut App, should_follow_bottom: bool) {
    app.preview.turns = Default::default();
    app.preview.session_origin = None;
    app.preview.session_id = None;
    app.preview.selected_turn = None;
    app.preview.expanded_turn = None;
    app.preview.view = PreviewView::Plain;
    app.preview.list_scroll = 0;
    app.preview.detail_scroll = 0;
    app.preview.follow_bottom = should_follow_bottom;
    app.preview.follow_selection = true;
}

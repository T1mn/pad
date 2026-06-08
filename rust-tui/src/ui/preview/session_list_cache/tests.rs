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

use super::{App, Focus, NavMode};
use std::time::{Duration, Instant};

#[test]
fn double_i_toggles_between_tree_and_index_map() {
    let mut app = App::new(std::env::temp_dir(), None);
    let now = Instant::now();

    app.press_index_toggle_key_at(now);
    assert_eq!(app.nav_mode, NavMode::Tree);

    app.press_index_toggle_key_at(now + Duration::from_millis(200));
    assert_eq!(app.nav_mode, NavMode::IndexMap);
    assert_eq!(app.focus, Focus::IndexMap);

    app.press_index_toggle_key_at(now + Duration::from_millis(900));
    app.press_index_toggle_key_at(now + Duration::from_millis(1000));
    assert_eq!(app.nav_mode, NavMode::Tree);
    assert_eq!(app.focus, Focus::Tree);
}

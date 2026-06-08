use crate::app::App;
use crate::log_debug;
use crate::ui;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};

pub(super) fn clear_and_draw(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    if app.needs_clear {
        terminal.clear()?;
        app.needs_clear = false;
        app.dirty = true;
    }

    if app.dirty {
        let draw_started = Instant::now();
        terminal.draw(|f| ui::draw(f, app))?;
        app.last_draw_elapsed = draw_started.elapsed();
        app.frame_budget_exceeded = app.last_draw_elapsed >= Duration::from_millis(12);
        if app.frame_budget_exceeded {
            log_debug!(
                "ui.frame: draw_slow elapsed_ms={} detail={} dirty_sidebar={}/{}",
                app.last_draw_elapsed.as_millis(),
                app.preview.view == crate::model::PreviewView::SessionDetail,
                app.sidebar.sidebar_folders_dirty,
                app.sidebar.visible_sidebar_items_dirty
            );
        }
        app.dirty = false;
    }

    Ok(())
}

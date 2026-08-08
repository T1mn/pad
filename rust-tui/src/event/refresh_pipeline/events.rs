use crate::app::App;

pub(super) fn drain_hook_events(app: &mut App) {
    let mut pending_hook_events = Vec::new();
    if let Some(ref mut hook_rx) = app.hook_rx {
        loop {
            match hook_rx.try_recv() {
                Ok(ev) => pending_hook_events.push(ev),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    app.hook_rx = None;
                    break;
                }
            }
        }
    }
    for ev in pending_hook_events {
        if app.should_defer_ui_updates() {
            app.deferred_hook_events.push(ev);
        } else {
            app.apply_hook_event(ev);
        }
    }
}

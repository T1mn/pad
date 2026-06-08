use super::super::text::format_two_sided;
use crate::app::App;
use crate::i18n::t;
use crate::model::PreviewSource;

pub(in crate::ui::status_bar) fn compose_status_body(app: &App, width: u16) -> String {
    let l = app.locale;
    let elapsed = app.last_refresh.elapsed().as_secs();
    let scan_status = if app.scan_in_progress {
        format!(" {}", t(l, "status.scanning"))
    } else {
        String::new()
    };
    let left = if app.sidebar.show_tree {
        if let Some(path) = &app.preview.file_preview_path {
            format!("📁 {}", path.display())
        } else {
            t(l, "tree.explorer").to_string()
        }
    } else {
        format!("{}s {}{}", elapsed, t(l, "status.ago"), scan_status)
    };

    let right_hint = if app.sidebar.show_tree {
        t(l, "status.tree_nav")
    } else if app.preview_is_focused() {
        if app.preview.source == PreviewSource::Session && !app.preview.turns.is_empty() {
            t(l, "status.preview_session_nav")
        } else {
            t(l, "status.preview_nav")
        }
    } else {
        t(l, "status.normal_nav_panel")
    };

    let left_text = if app.sidebar.show_tree {
        format!(
            "{}  {}s {}{}",
            left,
            elapsed,
            t(l, "status.ago"),
            scan_status
        )
    } else {
        format!(" {}", left)
    };

    format_two_sided(&left_text, right_hint, width as usize)
}

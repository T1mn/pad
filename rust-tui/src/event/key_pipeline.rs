use crate::app::state::Mode;
use crate::app::App;
use crate::log_debug;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub(super) fn handle_key_event(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    state: &mut super::loop_state::LoopState,
    key: KeyEvent,
) -> io::Result<()> {
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }

    if super::input_clear::handle_shift_delete(app, key) {
        return Ok(());
    }

    if key.code == KeyCode::Char(' ')
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && relay_real_chat_provider_list_is_focused(app)
    {
        if let Some(agent_name) = selected_relay_agent_name(app).map(str::to_string) {
            app.trigger_provider_batch_test_for_agent(&agent_name);
        }
        app.dirty = true;
        return Ok(());
    }

    match app.mode {
        Mode::Normal => {
            super::normal::handle_normal_mode(terminal, app, key)?;
        }
        Mode::Search => super::mode_dispatch::handle_search_mode(app, key.code),
        Mode::Settings => super::mode_dispatch::handle_settings_mode(app, key.code),
        Mode::Tree => super::mode_dispatch::handle_tree_mode(app, key.code),
        Mode::TreeSearch => super::mode_dispatch::handle_tree_search_mode(app, key.code),
        Mode::AgentLauncher => super::mode_dispatch::handle_agent_launcher_mode(app, key.code),
        Mode::DeleteConfirm => super::mode_dispatch::handle_delete_confirm_mode(app, key.code),
        Mode::ThreadActionConfirm => {
            super::mode_dispatch::handle_thread_action_confirm_mode(app, key)
        }
        Mode::Help => super::mode_dispatch::handle_help_mode(app, key.code),
        Mode::FuzzyPicker => super::mode_dispatch::handle_fuzzy_picker_mode(app, key),
        Mode::RelaySettings => super::mode_dispatch::handle_relay_settings_mode(app, key.code),
        Mode::FilePreview => super::mode_dispatch::handle_file_preview_mode(app, key.code),
        Mode::AgentStyleSettings => super::mode_dispatch::handle_agent_style_mode(app, key.code),
        Mode::TelegramSettings => {
            super::mode_dispatch::handle_telegram_settings_mode(app, key.code)
        }
        Mode::NotificationInbox => {
            super::mode_dispatch::handle_notification_inbox_mode(app, key.code)
        }
    }

    if matches!(
        key.code,
        KeyCode::Enter
            | KeyCode::Esc
            | KeyCode::Tab
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Char('h')
            | KeyCode::Char('j')
            | KeyCode::Char('k')
            | KeyCode::Char('J')
            | KeyCode::Char('K')
            | KeyCode::Char('l')
            | KeyCode::Char(' ')
    ) {
        let dropped = super::mouse::drain_pending_scroll_events(&mut state.carried_event)?;
        if dropped > 0 {
            log_debug!("input: dropped_pending_scroll_events count={}", dropped);
        }
    }

    Ok(())
}

pub(super) fn handle_paste(app: &mut App, text: &str) {
    if app.relay_popup_editing {
        app.relay_popup_buffer.push_str(text);
        app.dirty = true;
    } else if app.relay_editing {
        app.relay_edit_buffer.push_str(text);
        app.dirty = true;
    } else if app.telegram_editing {
        app.telegram_edit_buffer.push_str(text);
        app.dirty = true;
    } else if app.sidebar.thread_meta_editing {
        app.sidebar.thread_meta_buffer.push_str(text);
        app.dirty = true;
    } else if app.mode == Mode::Settings && app.settings_searching {
        app.settings_search.push_str(text);
        app.settings_selected = 0;
        app.dirty = true;
    } else if app.mode == Mode::Search {
        app.search_query.push_str(text);
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();
        app.dirty = true;
    }
}

fn relay_real_chat_provider_list_is_focused(app: &App) -> bool {
    use crate::app::state::{Mode, RelayView, SettingsDetailKind, SettingsFocus};

    let relay_is_visible = match app.mode {
        Mode::RelaySettings => true,
        Mode::Settings => {
            app.settings_focus == SettingsFocus::Detail
                && app.current_settings_detail_kind() == Some(SettingsDetailKind::Relay)
        }
        _ => false,
    };
    relay_is_visible
        && app.relay_view == RelayView::ProviderList
        && app
            .config
            .agents
            .get(app.relay_selected_agent)
            .is_some_and(|agent| matches!(agent.name.as_str(), "claude" | "codex"))
}

fn selected_relay_agent_name(app: &App) -> Option<&str> {
    app.config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.name.as_str())
}

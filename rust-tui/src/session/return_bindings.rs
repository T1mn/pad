mod context {
    pub(in crate::session) struct ReturnBindingContext<'a> {
        pub(in crate::session) trace_id: &'a str,
        pub(in crate::session) target_session: &'a str,
        pub(in crate::session) target_window: &'a str,
        pub(in crate::session) pad_pane: &'a str,
        pub(in crate::session) pad_window: &'a str,
        pub(in crate::session) pad_session: &'a str,
        pub(in crate::session) status_restore_value: Option<&'a str>,
    }
}
mod install {
    use crate::app::App;
    use std::process::Command;

    use super::super::bindings::{pad_sider_toggle_command, PAD_SIDER_TOGGLE_KEYS};
    use super::super::shell::wrap_tmux_run_shell;
    use super::context::ReturnBindingContext;
    use super::return_cmd::build_return_run_shell_cmd;

    pub(in crate::session) fn install_return_bindings(
        app: &mut App,
        ctx: &ReturnBindingContext<'_>,
    ) {
        let run_shell_cmd = build_return_run_shell_cmd(app, ctx);
        let bind_result = Command::new("tmux")
            .args(["bind-key", "-T", "root", "F12", "run-shell", &run_shell_cmd])
            .output();
        log_debug!(
            "handoff trace={} stage=create.bind_installed cmd={} result={:?}",
            ctx.trace_id,
            run_shell_cmd,
            bind_result.map(|o| o.status)
        );

        let _ = Command::new("tmux")
            .args(["bind-key", "-T", "root", "C-q", "run-shell", &run_shell_cmd])
            .output();
        let sider_cmd = wrap_tmux_run_shell(&pad_sider_toggle_command());
        for key in PAD_SIDER_TOGGLE_KEYS {
            let _ = Command::new("tmux")
                .args(["bind-key", "-T", "root", key, "run-shell", &sider_cmd])
                .output();
        }

        app.same_session_attached = true;
        log_debug!(
            "handoff trace={} stage=create.same_session_attached",
            ctx.trace_id
        );
    }
}
mod return_cmd;
mod saved {
    use crate::app::App;

    use super::super::bindings::{
        current_root_binding, restore_binding_cmd, PAD_SIDER_TOGGLE_KEYS,
    };

    pub(in crate::session) fn save_current_return_bindings(app: &mut App) {
        app.saved_tmux_bindings.clear();
        if let Some(line) = current_root_binding("F12") {
            app.saved_tmux_bindings.push(line);
        }
        if let Some(line) = current_root_binding("C-q") {
            app.saved_tmux_bindings.push(line);
        }
        for key in PAD_SIDER_TOGGLE_KEYS {
            if let Some(line) = current_root_binding(key) {
                app.saved_tmux_bindings.push(line);
            }
        }
    }

    pub(super) fn saved_binding_restore_cmd(app: &App, key: &str) -> String {
        restore_binding_cmd(
            app.saved_tmux_bindings
                .iter()
                .find(|line| line.contains(&format!(" {} ", key)))
                .map(String::as_str),
            key,
        )
    }
}

pub(super) use context::ReturnBindingContext;
pub(super) use install::install_return_bindings;
pub(super) use saved::save_current_return_bindings;

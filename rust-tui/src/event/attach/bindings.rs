mod install;
mod restore;

#[cfg(test)]
pub(super) use crate::tmux_bindings::{restore_binding_cmd, PAD_SIDER_TOGGLE_KEYS};
pub(super) use install::install_return_bindings;
pub(crate) use restore::restore_tmux_bindings;

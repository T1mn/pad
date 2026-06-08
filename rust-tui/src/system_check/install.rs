mod detect;
mod model;
mod steps;

#[cfg(test)]
pub(super) use detect::detect_install_plan_for;
pub(super) use detect::{detect_install_plan, tmux_exists};
pub(super) use model::InstallPlan;
pub(super) use steps::install_tmux;

use super::InstallPlan;
use std::process::Command;

pub(in crate::system_check) fn tmux_exists() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub(in crate::system_check) fn detect_install_plan() -> Option<InstallPlan> {
    detect_install_plan_for(std::env::consts::OS, &command_exists)
}

pub(in crate::system_check) fn detect_install_plan_for(
    os: &str,
    command_exists: &dyn Fn(&str) -> bool,
) -> Option<InstallPlan> {
    match os {
        "macos" if command_exists("brew") => Some(InstallPlan::Brew),
        "linux" => {
            if command_exists("apt-get") {
                Some(InstallPlan::Apt)
            } else if command_exists("dnf") {
                Some(InstallPlan::Dnf)
            } else if command_exists("yum") {
                Some(InstallPlan::Yum)
            } else if command_exists("pacman") {
                Some(InstallPlan::Pacman)
            } else if command_exists("zypper") {
                Some(InstallPlan::Zypper)
            } else if command_exists("apk") {
                Some(InstallPlan::Apk)
            } else if command_exists("brew") {
                Some(InstallPlan::Brew)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .args(["-lc", &format!("command -v {command} >/dev/null 2>&1")])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

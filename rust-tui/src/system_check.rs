use crate::tmux_capabilities::{probe_tmux_capabilities, TmuxProbeReport};
use std::io::{self, IsTerminal, Write};
use std::process::Command;

pub fn ensure_tmux_available() -> io::Result<TmuxProbeReport> {
    if tmux_exists() {
        return ensure_tmux_compatible();
    }

    let Some(plan) = detect_install_plan() else {
        return Err(io::Error::other(
            "PAD 需要 tmux，但当前未检测到可用的安装方式。请先手动安装 tmux 后重试。",
        ));
    };

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(format!(
            "PAD 需要 tmux。请先执行 `{}` 后重试。",
            plan.manual_hint()
        )));
    }

    eprintln!("PAD 需要 tmux，但当前系统未检测到 `tmux`。");
    eprintln!("是否现在自动安装？ [y/N]");
    eprint!("> ");
    io::stderr().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if !is_yes_answer(&answer) {
        return Err(io::Error::other(format!(
            "已取消安装。请先执行 `{}` 后重试。",
            plan.manual_hint()
        )));
    }

    eprintln!("开始安装 tmux...");
    install_tmux(&plan)?;

    if tmux_exists() {
        eprintln!("tmux 安装完成。");
        ensure_tmux_compatible()
    } else {
        Err(io::Error::other(
            "tmux 安装流程已执行，但仍未检测到 tmux，请手动检查系统环境。",
        ))
    }
}

pub fn tmux_doctor() -> io::Result<TmuxProbeReport> {
    if !tmux_exists() {
        return Err(io::Error::other(
            "tmux is not installed or not available in PATH.",
        ));
    }
    ensure_tmux_compatible()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstallPlan {
    Brew,
    Apt,
    Dnf,
    Yum,
    Pacman,
    Zypper,
    Apk,
}

impl InstallPlan {
    fn manual_hint(self) -> &'static str {
        match self {
            Self::Brew => "brew install tmux",
            Self::Apt => "sudo apt-get update && sudo apt-get install -y tmux",
            Self::Dnf => "sudo dnf install -y tmux",
            Self::Yum => "sudo yum install -y tmux",
            Self::Pacman => "sudo pacman -Sy --noconfirm tmux",
            Self::Zypper => "sudo zypper --non-interactive install tmux",
            Self::Apk => "sudo apk add tmux",
        }
    }
}

fn tmux_exists() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn detect_install_plan() -> Option<InstallPlan> {
    detect_install_plan_for(std::env::consts::OS, &command_exists)
}

fn detect_install_plan_for(os: &str, command_exists: &dyn Fn(&str) -> bool) -> Option<InstallPlan> {
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

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .args(["-lc", &format!("command -v {command} >/dev/null 2>&1")])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn install_tmux(plan: &InstallPlan) -> io::Result<()> {
    for (program, args) in install_steps(*plan) {
        let status = Command::new(program).args(args).status()?;
        if !status.success() {
            return Err(io::Error::other(format!(
                "tmux 安装失败，请手动执行 `{}`。",
                plan.manual_hint()
            )));
        }
    }
    Ok(())
}

fn ensure_tmux_compatible() -> io::Result<TmuxProbeReport> {
    let report = probe_tmux_capabilities().map_err(io::Error::other)?;
    let required = report.missing_required_capabilities();
    if required.is_empty() {
        return Ok(report);
    }

    let version = report.version_raw.trim();
    let mut message = format!(
        "tmux compatibility probe failed for `{}`. Missing required capabilities: {}.",
        version,
        required.join(", ")
    );
    let optional = report.missing_optional_capabilities();
    if !optional.is_empty() {
        message.push_str(&format!(
            " Optional capabilities unavailable: {}.",
            optional.join(", ")
        ));
    }
    if !report.notes.is_empty() {
        message.push_str(&format!(" Probe notes: {}.", report.notes.join(" | ")));
    }
    Err(io::Error::other(message))
}

fn install_steps(plan: InstallPlan) -> Vec<(&'static str, Vec<&'static str>)> {
    let use_sudo = command_exists("sudo");
    match plan {
        InstallPlan::Brew => vec![("brew", vec!["install", "tmux"])],
        InstallPlan::Apt => {
            if use_sudo {
                vec![
                    ("sudo", vec!["apt-get", "update"]),
                    ("sudo", vec!["apt-get", "install", "-y", "tmux"]),
                ]
            } else {
                vec![
                    ("apt-get", vec!["update"]),
                    ("apt-get", vec!["install", "-y", "tmux"]),
                ]
            }
        }
        InstallPlan::Dnf => {
            if use_sudo {
                vec![("sudo", vec!["dnf", "install", "-y", "tmux"])]
            } else {
                vec![("dnf", vec!["install", "-y", "tmux"])]
            }
        }
        InstallPlan::Yum => {
            if use_sudo {
                vec![("sudo", vec!["yum", "install", "-y", "tmux"])]
            } else {
                vec![("yum", vec!["install", "-y", "tmux"])]
            }
        }
        InstallPlan::Pacman => {
            if use_sudo {
                vec![("sudo", vec!["pacman", "-Sy", "--noconfirm", "tmux"])]
            } else {
                vec![("pacman", vec!["-Sy", "--noconfirm", "tmux"])]
            }
        }
        InstallPlan::Zypper => {
            if use_sudo {
                vec![(
                    "sudo",
                    vec!["zypper", "--non-interactive", "install", "tmux"],
                )]
            } else {
                vec![("zypper", vec!["--non-interactive", "install", "tmux"])]
            }
        }
        InstallPlan::Apk => {
            if use_sudo {
                vec![("sudo", vec!["apk", "add", "tmux"])]
            } else {
                vec![("apk", vec!["add", "tmux"])]
            }
        }
    }
}

fn is_yes_answer(input: &str) -> bool {
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_install_plan_prefers_apt_on_linux() {
        let exists = |command: &str| matches!(command, "apt-get" | "dnf");
        assert_eq!(
            detect_install_plan_for("linux", &exists),
            Some(InstallPlan::Apt)
        );
    }

    #[test]
    fn detect_install_plan_uses_brew_on_macos() {
        let exists = |command: &str| command == "brew";
        assert_eq!(
            detect_install_plan_for("macos", &exists),
            Some(InstallPlan::Brew)
        );
    }

    #[test]
    fn yes_answer_accepts_short_and_long_form() {
        assert!(is_yes_answer("y"));
        assert!(is_yes_answer("YES"));
        assert!(!is_yes_answer("n"));
        assert!(!is_yes_answer(""));
    }
}

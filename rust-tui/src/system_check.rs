mod compatibility;
mod install;

#[cfg(test)]
mod tests;

use crate::tmux_capabilities::TmuxProbeReport;
use std::io::{self, IsTerminal, Write};

use compatibility::ensure_tmux_compatible;
use install::{detect_install_plan, install_tmux, tmux_exists};

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
    install_tmux(plan)?;

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

fn is_yes_answer(input: &str) -> bool {
    let input = input.trim();
    input.eq_ignore_ascii_case("y") || input.eq_ignore_ascii_case("yes")
}

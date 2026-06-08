use std::error::Error;

pub fn is_info_only_command(args: &[String]) -> bool {
    args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--help" | "-h" | "--version" | "-V" | "--tmux-doctor"
        )
    })
}

pub fn is_telegram_daemon_command(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "telegram-bot")
}

pub fn is_internal_command(args: &[String]) -> bool {
    matches!(args.get(1).map(String::as_str), Some("__internal"))
}

pub fn run_internal_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    match args.get(2).map(String::as_str) {
        Some("pad-sider") => crate::pad_sider::run_args(args.iter().skip(3).cloned()),
        Some("workspace-recipe") => crate::workspace_recipe::run_args(args.iter().skip(3).cloned()),
        Some("browser-remote") => crate::browser_remote::run_args(args.iter().skip(3).cloned()),
        Some("agent-resume") => crate::agent_resume::run_args(args.iter().skip(3).cloned()),
        Some("socket-api") => crate::socket_api::run_args(args.iter().skip(3).cloned()),
        Some("codex-turn-diff") => crate::codex_turn_diff::run_args(args.iter().skip(3).cloned()),
        Some(other) => Err(format!("unknown internal command: {other}").into()),
        None => Err("missing internal command".into()),
    }
}

pub fn handle_info_command(args: &[String]) -> Result<bool, Box<dyn Error>> {
    if !is_info_only_command(args) {
        return Ok(false);
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(true);
    }

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("pad {}", env!("CARGO_PKG_VERSION"));
        return Ok(true);
    }

    if args.iter().any(|a| a == "--tmux-doctor") {
        let report = crate::system_check::tmux_doctor()?;
        for line in report.summary_lines() {
            println!("{line}");
        }
        let required = report.missing_required_capabilities();
        if !required.is_empty() {
            println!("required missing: {}", required.join(", "));
        }
        let optional = report.missing_optional_capabilities();
        if !optional.is_empty() {
            println!("optional missing: {}", optional.join(", "));
        }
        return Ok(true);
    }

    Ok(false)
}

fn print_help() {
    println!("PAD - Panel for Agent Development");
    println!();
    println!("Usage: pad [OPTIONS]");
    println!("       pad telegram-bot");
    println!();
    println!("Options:");
    println!("  -h, --help     Show help");
    println!("  -V, --version  Show version");
    println!("  -d, --debug    Enable debug logging (~/.pad/logs/pad.log)");
    println!("      --tmux-doctor  Probe tmux compatibility and print capability details");
    println!();
    println!("Key bindings:");
    println!("  j/k or ↑/↓     Move selection");
    println!("  1-9            Jump to visible session");
    println!("  Enter          Attach to panel (F12 / Ctrl+Q to return)");
    println!("  t              Toggle file tree");
    println!("  Space          Expand or collapse directory");
    println!("  /              Open settings");
    println!("  ?              Help");
    println!("  r              Refresh");
    println!("  c              Create session");
    println!("  d              Delete pane + hide thread");
    println!("  F1             Settings");
    println!("  q              Quit");
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;

use std::error::Error;

pub fn is_info_only_command(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h" | "--version" | "-V"))
}

pub fn is_telegram_daemon_command(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "telegram-bot")
}

pub fn is_internal_command(args: &[String]) -> bool {
    matches!(args.get(1).map(String::as_str), Some("__internal"))
}

pub fn run_internal_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    match args.get(2).map(String::as_str) {
        Some("browser-remote") => crate::browser_remote::run_args(args.iter().skip(3).cloned()),
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
    println!("                    PAD always uses its built-in native terminal runtime");
    println!();
    println!("Key bindings:");
    println!("  j/k or ↑/↓     Move selection");
    println!("  1-9            Jump to visible session");
    println!("  Tab            Focus the native terminal (F12 to return)");
    println!("  t              Toggle file tree");
    println!("  Space          Expand or collapse directory");
    println!("  /              Open settings");
    println!("  ?              Help");
    println!("  r              Refresh");
    println!("  c              Choose a directory and launch an agent terminal");
    println!("  d              Delete pane + hide thread");
    println!("  F1             Settings");
    println!("  q              Quit");
}

#[cfg(test)]
#[path = "cli_tests.rs"]
pub(crate) mod tests;

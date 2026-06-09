use super::{browser_open_command, open_browser_url, remote_ssh_command, RemoteCommandRequest};
use std::error::Error;
use std::process::Command;

pub fn run_args(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args.into_iter().collect();
    match args.first().map(String::as_str) {
        Some("browser-open") => run_browser_open(&args[1..]),
        Some("remote-exec") => run_remote_exec(&args[1..]),
        Some(other) => Err(format!("unknown browser-remote command: {other}").into()),
        None => Err("usage: pad __internal browser-remote browser-open <url> [--dry-run]".into()),
    }
}

fn run_browser_open(args: &[String]) -> Result<(), Box<dyn Error>> {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let url = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .ok_or("missing url")?;
    if dry_run {
        let command = browser_open_command(url)?;
        println!(
            "{}",
            format_command_line(&command.program, command.args.iter().map(String::as_str))
        );
        return Ok(());
    }
    open_browser_url(url)?;
    println!("opened {url}");
    Ok(())
}

fn run_remote_exec(args: &[String]) -> Result<(), Box<dyn Error>> {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let host = value_after(args, "--host").ok_or("missing --host")?;
    let cwd = value_after(args, "--cwd");
    let sep = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or("missing -- command")?;
    let command = format_args(&args[sep + 1..]);
    if command.trim().is_empty() {
        return Err("missing remote command".into());
    }
    let ssh = remote_ssh_command(&RemoteCommandRequest { host, cwd, command });
    if dry_run {
        println!("{}", format_args(&ssh));
        return Ok(());
    }
    let output = Command::new(&ssh[0]).args(&ssh[1..]).output()?;
    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
        return Ok(());
    }
    Err(format!(
        "remote exec failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}

fn value_after(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == key)
        .map(|pair| pair[1].clone())
}

fn format_args(args: &[String]) -> String {
    format_command_line("", args.iter().map(String::as_str))
}

fn format_command_line<'a>(program: &str, args: impl IntoIterator<Item = &'a str>) -> String {
    let mut line = program.to_string();
    for arg in args {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(arg);
    }
    line
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;

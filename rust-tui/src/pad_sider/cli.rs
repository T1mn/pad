use std::path::PathBuf;

pub enum Command {
    Toggle {
        target_pane: String,
    },
    Ui {
        cwd: PathBuf,
        target_pane: Option<String>,
    },
}

pub fn parse<I>(mut args: I) -> Result<Command, String>
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        Some("toggle") => parse_toggle(args),
        Some("ui") => parse_ui(args),
        Some(other) => Err(format!("unknown command: {other}")),
        None => Err("missing command: expected `toggle` or `ui`".into()),
    }
}

fn parse_toggle<I>(mut args: I) -> Result<Command, String>
where
    I: Iterator<Item = String>,
{
    let mut target_pane = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--target-pane" => target_pane = args.next(),
            other => return Err(format!("unknown toggle arg: {other}")),
        }
    }
    let target_pane = target_pane.ok_or_else(|| "missing --target-pane".to_string())?;
    Ok(Command::Toggle { target_pane })
}

fn parse_ui<I>(mut args: I) -> Result<Command, String>
where
    I: Iterator<Item = String>,
{
    let mut cwd = None;
    let mut target_pane = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cwd" => cwd = args.next().map(PathBuf::from),
            "--target-pane" => target_pane = args.next(),
            other => return Err(format!("unknown ui arg: {other}")),
        }
    }
    let cwd = cwd.ok_or_else(|| "missing --cwd".to_string())?;
    Ok(Command::Ui { cwd, target_pane })
}

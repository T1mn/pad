mod actions;
mod app;
mod cli;
mod fs;
mod index_map;
mod layout;
mod preview;
mod preview_cache;
mod search;
mod sizing;
mod tmux;
mod tree;
mod ui;

pub fn run_args<I>(args: I) -> Result<(), Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    match cli::parse(args)? {
        cli::Command::Toggle { target_pane } => tmux::toggle(&target_pane).map_err(Into::into),
        cli::Command::Ui { cwd, target_pane } => ui::run(cwd, target_pane).map_err(Into::into),
    }
}

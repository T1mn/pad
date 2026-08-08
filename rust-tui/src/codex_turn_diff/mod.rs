mod git;
mod model;
mod recorder;
mod storage;
mod storage_paths;

pub use recorder::record_codex_hook_event;

use crate::hook::HookEvent;
use std::io::{self, Read};

pub fn run_args<I>(mut args: I) -> Result<(), Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        Some("hook") => {
            let mut raw = String::new();
            io::stdin().read_to_string(&mut raw)?;
            let event: HookEvent = serde_json::from_str(&raw)?;
            record_codex_hook_event(&event)?;
            Ok(())
        }
        Some(other) => Err(format!("unknown codex-turn-diff command: {other}").into()),
        None => Err("usage: pad __internal codex-turn-diff hook < event.json".into()),
    }
}

#[cfg(test)]
mod tests;

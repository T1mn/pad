mod command;
mod filter;
mod parse;

pub(super) use command::{load_lightweight_process_snapshot, load_process_snapshot};
use std::collections::HashMap;

pub(super) type ProcessMaps = (HashMap<String, String>, HashMap<String, Vec<String>>);

#[cfg(test)]
#[path = "loader_tests.rs"]
mod tests;

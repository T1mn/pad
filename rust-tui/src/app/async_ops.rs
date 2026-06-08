use crate::model::AgentPanel;
use std::error::Error;

mod codex_cli;
mod preview_detail;
mod preview_update;
mod provider_test;
mod scan;
mod title_summary;

pub use codex_cli::CodexCliVersionInfo;
pub(crate) use codex_cli::{CodexCliUpdateResult, CodexCliVersionCheckResult};
pub(crate) use provider_test::ProviderTestResult;

/// Async scan result channel type
pub type ScanResult = Result<Vec<AgentPanel>, Box<dyn Error + Send + Sync>>;

#[cfg(test)]
mod async_ops_tests;

mod normalize;
mod prompt;
mod request;
mod response;
mod types;
mod util;
mod window;

pub use normalize::normalize_generated_title;
pub use request::request_title_summary;
pub use types::{SummaryWireApi, TitleSummaryResult};
pub use window::{is_enabled, select_turn_window, should_refresh_title};

#[cfg(test)]
mod tests;

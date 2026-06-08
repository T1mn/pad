mod build;
mod matchers;
mod range;
mod visible;

pub(crate) use build::ensure_session_list_cache;
pub(crate) use range::selected_session_list_range;
pub(crate) use visible::visible_session_list_lines;

#[cfg(test)]
#[path = "session_list_cache/tests.rs"]
mod tests;

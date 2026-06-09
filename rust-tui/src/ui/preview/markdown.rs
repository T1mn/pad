mod inline;
mod normalize;
mod render;
mod style;
mod wrap;

pub(crate) use inline::{format_line, retokenize_inline_code, tokenize_inline_code};
pub(crate) use normalize::normalize_session_detail_markdown;
pub(crate) use render::{
    detail_surface, render_detail_content_line, render_detail_padding_line,
    render_detail_separator_line,
};
pub(crate) use style::markdown_options;
pub(crate) use wrap::{
    flatten_lines_for_smooth_scrolling, total_span_count, wrap_styled_line, wrap_text_to_width,
};

#[cfg(test)]
#[path = "markdown/normalize_tests.rs"]
mod normalize_tests;

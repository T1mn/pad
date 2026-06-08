mod events;
mod render;
mod style;

pub use render::render_markdown;

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;

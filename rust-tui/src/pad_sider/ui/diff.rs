mod model;
mod parse;
mod render;
mod side_by_side;
mod styles;
mod unified;

pub use render::render_diff_patch;

#[cfg(test)]
mod tests;

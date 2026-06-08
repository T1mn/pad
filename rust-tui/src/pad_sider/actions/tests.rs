use super::super::app::{App, Focus, NavMode};
use std::fs;
use std::time::{Duration, Instant};

mod display {
    use super::*;
    include!("tests/display.rs");
}

mod index_map {
    use super::*;
    include!("tests/index_map.rs");
}

mod preview {
    use super::*;
    include!("tests/preview.rs");
}

mod support {
    include!("tests/support.rs");
}

mod tick {
    use super::*;
    include!("tests/tick.rs");
}

mod tree {
    use super::*;
    include!("tests/tree.rs");
}

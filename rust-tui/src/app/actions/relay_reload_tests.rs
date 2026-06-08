use super::*;
use crate::app::state::RelayView;
use crate::theme::{Config, ProviderConfig};

mod deferred {
    use super::*;
    include!("relay_reload_tests/deferred.rs");
}

mod invalid {
    use super::*;
    include!("relay_reload_tests/invalid.rs");
}

mod immediate {
    use super::*;
    include!("relay_reload_tests/immediate.rs");
}

mod selection {
    use super::*;
    include!("relay_reload_tests/selection.rs");
}

mod support {
    use super::*;
    include!("relay_reload_tests/support.rs");
}

use super::*;
use crate::hook::HookTmuxInfo;

mod completion {
    use super::*;
    include!("tests/completion.rs");
}

mod phase_gate {
    use super::*;
    include!("tests/phase_gate.rs");
}

mod support {
    use super::*;
    include!("tests/support.rs");
}

mod turn_match {
    use super::*;
    include!("tests/turn_match.rs");
}

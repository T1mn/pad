mod claude {
    use super::*;
    include!("provider_configs/claude.rs");
}

mod claude_safety {
    use super::*;
    include!("provider_configs/claude_safety.rs");
}

mod codex {
    use super::*;
    include!("provider_configs/codex.rs");
}

mod deepseek {
    use super::*;
    include!("provider_configs/deepseek.rs");
}

mod gemini {
    use super::*;
    include!("provider_configs/gemini.rs");
}

mod opencode {
    use super::*;
    include!("provider_configs/opencode.rs");
}

mod opencode_safety {
    use super::*;
    include!("provider_configs/opencode_safety.rs");
}

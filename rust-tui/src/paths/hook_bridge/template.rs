mod base;
mod builder;
mod options {
    pub(super) struct HookBridgeTemplateOptions {
        pub(super) version: &'static str,
        pub(super) silence_stdio_block: &'static str,
        pub(super) load_payload_block: &'static str,
        pub(super) main_start_line: &'static str,
        pub(super) payload_expr: &'static str,
        pub(super) hook_type_line: &'static str,
        pub(super) event_name_expr: &'static str,
        pub(super) record_turn_diff_block: String,
    }
}

pub(in crate::paths) use builder::{claude_hook_bridge_template, codex_hook_bridge_template};

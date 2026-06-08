pub(super) struct HookBridgeTemplateOptions {
    pub(super) version: &'static str,
    pub(super) silence_stdio_block: &'static str,
    pub(super) tmux_stderr_arg: &'static str,
    pub(super) load_payload_block: &'static str,
    pub(super) main_start_line: &'static str,
    pub(super) payload_expr: &'static str,
    pub(super) hook_type_line: &'static str,
    pub(super) event_name_expr: &'static str,
    pub(super) record_turn_diff_block: String,
}

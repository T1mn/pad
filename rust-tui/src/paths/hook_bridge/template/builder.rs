use super::super::{CLAUDE_BRIDGE_VERSION, CODEX_BRIDGE_VERSION};
use super::base::HOOK_BRIDGE_TEMPLATE_BASE;
use super::options::HookBridgeTemplateOptions;

pub(in crate::paths) fn claude_hook_bridge_template() -> String {
    build_hook_bridge_template(HookBridgeTemplateOptions {
        version: CLAUDE_BRIDGE_VERSION,
        silence_stdio_block: "",
        tmux_stderr_arg: "",
        load_payload_block: "",
        main_start_line: "    raw = sys.stdin.read()",
        payload_expr: "json.loads(raw)",
        hook_type_line: "",
        event_name_expr: "payload.get(\"hook_event_name\")",
        record_turn_diff_block: "def record_codex_turn_diff(message):\n    pass\n".into(),
    })
}

pub(in crate::paths) fn codex_hook_bridge_template() -> String {
    let pad_binary = std::env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    build_hook_bridge_template(HookBridgeTemplateOptions {
        version: CODEX_BRIDGE_VERSION,
        silence_stdio_block: r#"def silence_stdio():
    devnull = open(os.devnull, "w")
    sys.stdout = devnull
    sys.stderr = devnull
"#,
        tmux_stderr_arg: ",\n            stderr=subprocess.DEVNULL",
        load_payload_block: r#"def load_payload():
    if sys.stdin.isatty():
        return {}
    try:
        return json.load(sys.stdin)
    except Exception:
        return {}


def pad_codex_hooks_enabled():
    if os.environ.get("PAD_CODEX_HOOKS") == "1":
        return True
    legacy_home = os.environ.get("CODEX_HOME", "")
    return legacy_home.endswith("/.pad/codex-home")
"#,
        main_start_line: "    silence_stdio()",
        payload_expr: "load_payload()",
        hook_type_line:
            "    hook_type = sys.argv[1] if len(sys.argv) > 1 else payload.get(\"hook_event_name\")",
        event_name_expr: "payload.get(\"hook_event_name\") or hook_type",
        record_turn_diff_block: format!(
            r#"PAD_BINARY = {pad_binary:?}


def record_codex_turn_diff(message):
    if message.get("event") not in ("user_prompt_submit", "stop"):
        return
    if not PAD_BINARY:
        return
    try:
        subprocess.run(
            [PAD_BINARY, "__internal", "codex-turn-diff", "hook"],
            input=json.dumps(message, ensure_ascii=False),
            text=True,
            timeout=12,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass
"#
        ),
    })
}

fn build_hook_bridge_template(options: HookBridgeTemplateOptions) -> String {
    HOOK_BRIDGE_TEMPLATE_BASE
        .replace("__PAD_BRIDGE_VERSION__", options.version)
        .replace("__PAD_SILENCE_STDIO_BLOCK__", options.silence_stdio_block)
        .replace("__PAD_TMUX_STDERR_ARG__", options.tmux_stderr_arg)
        .replace("__PAD_LOAD_PAYLOAD_BLOCK__", options.load_payload_block)
        .replace("__PAD_MAIN_START_LINE__", options.main_start_line)
        .replace("__PAD_PAYLOAD_EXPR__", options.payload_expr)
        .replace("__PAD_HOOK_TYPE_LINE__", options.hook_type_line)
        .replace("__PAD_EVENT_NAME_EXPR__", options.event_name_expr)
        .replace(
            "__PAD_RECORD_TURN_DIFF_BLOCK__",
            &options.record_turn_diff_block,
        )
}

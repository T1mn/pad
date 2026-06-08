#[derive(Debug, Default)]
pub(super) struct CodexRelayExport {
    pub(super) version: u32,
    pub(super) codex: CodexRelayConfig,
}

#[derive(Debug, Default)]
pub(super) struct CodexRelayConfig {
    pub(super) active_provider: Option<usize>,
    pub(super) providers: Vec<CodexRelayProvider>,
}

#[derive(Debug, Default)]
pub(super) struct CodexRelayProvider {
    pub(super) label: String,
    pub(super) provider_name: String,
    pub(super) base_url: String,
    pub(super) api_key: String,
    pub(super) env_key: String,
}

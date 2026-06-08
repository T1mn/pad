#[derive(Clone, Debug, Default)]
pub struct CodexCliVersionInfo {
    pub binary_path: Option<String>,
    pub local_version: Option<String>,
    pub latest_version: Option<String>,
}

pub(crate) type CodexCliVersionCheckResult = CodexCliVersionInfo;
pub(crate) type CodexCliUpdateResult = Result<CodexCliVersionInfo, String>;

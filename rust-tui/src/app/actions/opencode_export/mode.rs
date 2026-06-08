#[derive(Clone, Copy)]
pub(in crate::app::actions::opencode_export) enum ExportMode {
    Raw,
    Sanitized,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TmuxCapabilities {
    pub pane_metadata_formats: bool,
    pub display_message_formats: bool,
    pub root_key_table: bool,
    pub literal_send_keys: bool,
    pub bracketed_paste: bool,
    pub control_mode_flags: bool,
    pub focus_events: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TmuxProbeReport {
    pub version_raw: String,
    pub version: Option<super::TmuxVersion>,
    pub capabilities: TmuxCapabilities,
    pub notes: Vec<String>,
}

impl TmuxProbeReport {
    pub fn missing_required_capabilities(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.capabilities.pane_metadata_formats {
            missing.push("pane metadata formats");
        }
        if !self.capabilities.display_message_formats {
            missing.push("display-message formats");
        }
        if !self.capabilities.root_key_table {
            missing.push("root key table bindings");
        }
        if !self.capabilities.literal_send_keys {
            missing.push("send-keys -l");
        }
        missing
    }

    pub fn missing_optional_capabilities(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.capabilities.control_mode_flags {
            missing.push("control-mode attach flags");
        }
        if !self.capabilities.bracketed_paste {
            missing.push("bracketed paste");
        }
        if !self.capabilities.focus_events {
            missing.push("focus-events");
        }
        missing
    }

    pub fn summary_lines(&self) -> Vec<String> {
        let version = self.version_raw.trim();
        let mut lines = Vec::with_capacity(2 + self.notes.len().saturating_add(1));
        lines.push(format!("tmux version: {}", version));
        let caps = &self.capabilities;
        lines.push(format!(
            "capabilities: pane-metadata={} display-message={} root-keys={} send-keys-l={} paste-p={} control-flags={} focus-events={}",
            yes_no(caps.pane_metadata_formats),
            yes_no(caps.display_message_formats),
            yes_no(caps.root_key_table),
            yes_no(caps.literal_send_keys),
            yes_no(caps.bracketed_paste),
            yes_no(caps.control_mode_flags),
            yes_no(caps.focus_events),
        ));
        if !self.notes.is_empty() {
            lines.push("notes:".to_string());
            for note in &self.notes {
                lines.push(format!("  - {}", note));
            }
        }
        lines
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

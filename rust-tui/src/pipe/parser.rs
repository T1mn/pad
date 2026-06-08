use super::TmuxEvent;

#[derive(Debug)]
pub(super) enum ParsedControlEvent<'a> {
    Emit {
        raw_type: &'a str,
        event: TmuxEvent,
    },
    Ignore {
        raw_type: &'a str,
        reason: &'static str,
    },
}

pub(super) fn parse_control_event(line: &str) -> Option<ParsedControlEvent<'_>> {
    let raw_type = line.split_whitespace().next()?;
    if !raw_type.starts_with('%') {
        return None;
    }

    match raw_type {
        "%window-add"
        | "%window-close"
        | "%window-renamed"
        | "%window-pane-changed"
        | "%layout-change" => Some(ParsedControlEvent::Emit {
            raw_type,
            event: TmuxEvent::WindowChanged,
        }),
        "%session-changed"
        | "%session-renamed"
        | "%sessions-changed"
        | "%session-window-changed" => Some(ParsedControlEvent::Emit {
            raw_type,
            event: TmuxEvent::SessionChanged,
        }),
        "%pane-mode-changed" => Some(ParsedControlEvent::Emit {
            raw_type,
            event: TmuxEvent::PaneModeChanged,
        }),
        "%output" | "%extended-output" => Some(ParsedControlEvent::Emit {
            raw_type,
            event: TmuxEvent::OutputDetected,
        }),
        "%begin" | "%end" | "%error" => Some(ParsedControlEvent::Ignore {
            raw_type,
            reason: "protocol frame",
        }),
        "%message"
        | "%client-detached"
        | "%client-session-changed"
        | "%config-error"
        | "%continue"
        | "%pause"
        | "%paste-buffer-changed"
        | "%paste-buffer-deleted"
        | "%subscription-changed"
        | "%unlinked-window-add"
        | "%unlinked-window-close"
        | "%unlinked-window-renamed"
        | "%exit" => Some(ParsedControlEvent::Ignore {
            raw_type,
            reason: "not relevant to panel scan",
        }),
        _ => Some(ParsedControlEvent::Ignore {
            raw_type,
            reason: "unrecognized control notification",
        }),
    }
}

use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PiEventKind {
    AgentStart,
    AgentEnd,
    AgentSettled,
    TurnStart,
    TurnEnd,
    MessageStart,
    MessageUpdate,
    MessageEnd,
    ToolExecutionStart,
    ToolExecutionUpdate,
    ToolExecutionEnd,
    QueueUpdate,
    CompactionStart,
    CompactionEnd,
    AutoRetryStart,
    AutoRetryEnd,
    ExtensionUiRequest,
    ExtensionError,
    Unknown,
}

impl PiEventKind {
    pub(crate) fn from_type(value: &str) -> Self {
        match value {
            "agent_start" => Self::AgentStart,
            "agent_end" => Self::AgentEnd,
            "agent_settled" => Self::AgentSettled,
            "turn_start" => Self::TurnStart,
            "turn_end" => Self::TurnEnd,
            "message_start" => Self::MessageStart,
            "message_update" => Self::MessageUpdate,
            "message_end" => Self::MessageEnd,
            "tool_execution_start" => Self::ToolExecutionStart,
            "tool_execution_update" => Self::ToolExecutionUpdate,
            "tool_execution_end" => Self::ToolExecutionEnd,
            "queue_update" => Self::QueueUpdate,
            "compaction_start" => Self::CompactionStart,
            "compaction_end" => Self::CompactionEnd,
            "auto_retry_start" => Self::AutoRetryStart,
            "auto_retry_end" => Self::AutoRetryEnd,
            "extension_ui_request" => Self::ExtensionUiRequest,
            "extension_error" => Self::ExtensionError,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PiRuntimeStatus {
    Starting,
    Idle,
    Running,
    Streaming,
    ToolRunning,
    NeedsApproval,
    NeedsInput,
    Compacting,
    Retrying,
    Failed,
    Disconnected,
}

impl Default for PiRuntimeStatus {
    fn default() -> Self {
        Self::Starting
    }
}

impl PiRuntimeStatus {
    pub(crate) fn is_busy(self) -> bool {
        matches!(
            self,
            Self::Starting
                | Self::Running
                | Self::Streaming
                | Self::ToolRunning
                | Self::Compacting
                | Self::Retrying
        )
    }

    pub(crate) fn needs_user_action(self) -> bool {
        matches!(self, Self::NeedsApproval | Self::NeedsInput | Self::Failed)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PiEvent {
    pub(crate) kind: PiEventKind,
    pub(crate) generation: Option<u64>,
    pub(crate) sequence: Option<u64>,
    pub(crate) value: Value,
}

impl PiEvent {
    pub(crate) fn parse(value: Value) -> Option<Self> {
        let object = value.as_object()?;
        let event_type = object.get("type").and_then(Value::as_str)?;
        Some(Self {
            kind: PiEventKind::from_type(event_type),
            generation: object.get("generation").and_then(Value::as_u64),
            sequence: object
                .get("sequence")
                .or_else(|| object.get("seq"))
                .and_then(Value::as_u64),
            value,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PiRuntimeSnapshot {
    pub(crate) generation: u64,
    pub(crate) status: PiRuntimeStatus,
    pub(crate) pending_message_count: usize,
    pub(crate) active_tool_call_id: Option<String>,
    pub(crate) last_sequence: Option<u64>,
}

impl Default for PiRuntimeSnapshot {
    fn default() -> Self {
        Self {
            generation: 0,
            status: PiRuntimeStatus::Starting,
            pending_message_count: 0,
            active_tool_call_id: None,
            last_sequence: None,
        }
    }
}

/// Small deterministic reducer for the events that affect PAD's task status.
/// In particular, `agent_end` is not treated as completion: Pi may retry,
/// compact, or consume a queued continuation after that low-level event.
#[derive(Clone, Debug, Default)]
pub(crate) struct PiEventReducer {
    snapshot: PiRuntimeSnapshot,
}

impl PiEventReducer {
    pub(crate) fn new(generation: u64) -> Self {
        Self {
            snapshot: PiRuntimeSnapshot {
                generation,
                ..PiRuntimeSnapshot::default()
            },
        }
    }

    pub(crate) fn snapshot(&self) -> &PiRuntimeSnapshot {
        &self.snapshot
    }

    pub(crate) fn apply(&mut self, event: PiEvent) -> bool {
        if event
            .generation
            .is_some_and(|generation| generation != self.snapshot.generation)
        {
            return false;
        }
        if let Some(sequence) = event.sequence {
            if self
                .snapshot
                .last_sequence
                .is_some_and(|previous| sequence <= previous)
            {
                return false;
            }
            self.snapshot.last_sequence = Some(sequence);
        }

        match event.kind {
            PiEventKind::AgentStart => self.snapshot.status = PiRuntimeStatus::Running,
            PiEventKind::AgentEnd => {
                // Deliberately keep the active state. `agent_settled` is the
                // only event that proves retries/compaction/queued work ended.
                if !self.snapshot.status.is_busy() {
                    self.snapshot.status = PiRuntimeStatus::Running;
                }
            }
            PiEventKind::AgentSettled => {
                self.snapshot.status = PiRuntimeStatus::Idle;
                self.snapshot.active_tool_call_id = None;
            }
            PiEventKind::TurnStart | PiEventKind::TurnEnd => {
                self.snapshot.status = PiRuntimeStatus::Running;
            }
            PiEventKind::MessageStart | PiEventKind::MessageUpdate => {
                self.snapshot.status = PiRuntimeStatus::Streaming;
            }
            PiEventKind::MessageEnd => self.snapshot.status = PiRuntimeStatus::Running,
            PiEventKind::ToolExecutionStart => {
                self.snapshot.status = PiRuntimeStatus::ToolRunning;
                self.snapshot.active_tool_call_id = event
                    .value
                    .get("toolCallId")
                    .or_else(|| event.value.get("tool_call_id"))
                    .or_else(|| event.value.get("callId"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            PiEventKind::ToolExecutionUpdate => {
                self.snapshot.status = PiRuntimeStatus::ToolRunning;
            }
            PiEventKind::ToolExecutionEnd => {
                self.snapshot.status = PiRuntimeStatus::Running;
                self.snapshot.active_tool_call_id = None;
            }
            PiEventKind::QueueUpdate => {
                self.snapshot.pending_message_count = event
                    .value
                    .get("pendingMessageCount")
                    .or_else(|| event.value.get("pending_message_count"))
                    .and_then(Value::as_u64)
                    .unwrap_or_else(|| {
                        event
                            .value
                            .get("queue")
                            .and_then(Value::as_array)
                            .map_or(0, Vec::len) as u64
                    }) as usize;
            }
            PiEventKind::CompactionStart => self.snapshot.status = PiRuntimeStatus::Compacting,
            PiEventKind::CompactionEnd => self.snapshot.status = PiRuntimeStatus::Running,
            PiEventKind::AutoRetryStart => self.snapshot.status = PiRuntimeStatus::Retrying,
            PiEventKind::AutoRetryEnd => self.snapshot.status = PiRuntimeStatus::Running,
            PiEventKind::ExtensionUiRequest => {
                let method = event
                    .value
                    .get("method")
                    .or_else(|| event.value.get("requestType"))
                    .and_then(Value::as_str);
                self.snapshot.status = match method {
                    Some("confirm" | "select") => PiRuntimeStatus::NeedsApproval,
                    Some("input" | "editor") => PiRuntimeStatus::NeedsInput,
                    _ => PiRuntimeStatus::NeedsApproval,
                };
            }
            PiEventKind::ExtensionError | PiEventKind::Unknown => {
                self.snapshot.status = PiRuntimeStatus::Failed;
            }
        }
        true
    }

    pub(crate) fn mark_disconnected(&mut self) {
        self.snapshot.status = PiRuntimeStatus::Disconnected;
        self.snapshot.active_tool_call_id = None;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn settled_is_the_only_completion_boundary() {
        let mut reducer = PiEventReducer::new(7);
        assert!(reducer.apply(PiEvent::parse(json!({"type":"agent_start"})).unwrap()));
        assert!(reducer.apply(PiEvent::parse(json!({"type":"agent_end"})).unwrap()));
        assert_ne!(reducer.snapshot().status, PiRuntimeStatus::Idle);
        assert!(reducer.apply(PiEvent::parse(json!({"type":"agent_settled"})).unwrap()));
        assert_eq!(reducer.snapshot().status, PiRuntimeStatus::Idle);
    }

    pub(crate) fn stale_generations_and_sequences_are_ignored() {
        let mut reducer = PiEventReducer::new(3);
        assert!(
            !reducer.apply(PiEvent::parse(json!({"type":"agent_start","generation":2})).unwrap())
        );
        assert!(reducer.apply(
            PiEvent::parse(json!({"type":"agent_start","generation":3,"sequence":4})).unwrap()
        ));
        assert!(!reducer.apply(
            PiEvent::parse(json!({"type":"agent_end","generation":3,"sequence":4})).unwrap()
        ));
        assert_eq!(reducer.snapshot().status, PiRuntimeStatus::Running);
    }

    pub(crate) fn approval_and_tool_events_update_runtime_status() {
        let mut reducer = PiEventReducer::new(1);
        assert!(reducer.apply(
            PiEvent::parse(json!({"type":"extension_ui_request","method":"confirm"})).unwrap()
        ));
        assert_eq!(reducer.snapshot().status, PiRuntimeStatus::NeedsApproval);
        assert!(reducer.apply(
            PiEvent::parse(json!({"type":"tool_execution_start","toolCallId":"c1"})).unwrap()
        ));
        assert_eq!(reducer.snapshot().status, PiRuntimeStatus::ToolRunning);
        assert_eq!(
            reducer.snapshot().active_tool_call_id.as_deref(),
            Some("c1")
        );
        reducer.mark_disconnected();
        assert_eq!(reducer.snapshot().status, PiRuntimeStatus::Disconnected);
    }
}

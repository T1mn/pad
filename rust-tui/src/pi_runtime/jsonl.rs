use serde::Serialize;
use serde_json::Value;
use std::fmt;

/// Pi's RPC transport is line-delimited JSON. Keep framing in a small codec so
/// partial PTY/pipe reads and malformed provider output never leak into the
/// runtime state reducer.
pub(crate) const DEFAULT_MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum JsonlError {
    FrameTooLarge { max_bytes: usize },
    CarriageReturnFraming,
    InvalidJson(String),
    IncompleteFrame,
    NonObjectMessage,
    MissingMessageType,
}

impl fmt::Display for JsonlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameTooLarge { max_bytes } => {
                write!(formatter, "Pi RPC frame exceeds {max_bytes} bytes")
            }
            Self::CarriageReturnFraming => {
                formatter.write_str("Pi RPC requires LF framing; CRLF is not accepted")
            }
            Self::InvalidJson(error) => write!(formatter, "invalid Pi RPC JSON: {error}"),
            Self::IncompleteFrame => formatter.write_str("incomplete Pi RPC frame"),
            Self::NonObjectMessage => formatter.write_str("Pi RPC message must be a JSON object"),
            Self::MissingMessageType => formatter.write_str("Pi RPC message is missing type"),
        }
    }
}

impl std::error::Error for JsonlError {}

#[derive(Clone, Debug)]
pub(crate) struct JsonlCodec {
    buffer: Vec<u8>,
    max_frame_bytes: usize,
}

impl Default for JsonlCodec {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_FRAME_BYTES)
    }
}

impl JsonlCodec {
    pub(crate) fn new(max_frame_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_frame_bytes: max_frame_bytes.max(1),
        }
    }

    /// Push an arbitrary transport chunk and return every complete message in
    /// order. The incomplete tail remains buffered for the next chunk.
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Result<Vec<Value>, JsonlError> {
        self.buffer.extend_from_slice(chunk);
        if !self.buffer.contains(&b'\n') && self.buffer.len() > self.max_frame_bytes {
            return Err(JsonlError::FrameTooLarge {
                max_bytes: self.max_frame_bytes,
            });
        }

        let mut messages = Vec::new();
        while let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut frame = self.buffer.drain(..=newline).collect::<Vec<_>>();
            frame.pop(); // LF is the only framing byte.
            if frame.len() > self.max_frame_bytes {
                return Err(JsonlError::FrameTooLarge {
                    max_bytes: self.max_frame_bytes,
                });
            }
            if frame.last() == Some(&b'\r') {
                return Err(JsonlError::CarriageReturnFraming);
            }
            if frame.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let value = serde_json::from_slice::<Value>(&frame)
                .map_err(|error| JsonlError::InvalidJson(error.to_string()))?;
            messages.push(value);
        }

        if self.buffer.len() > self.max_frame_bytes {
            return Err(JsonlError::FrameTooLarge {
                max_bytes: self.max_frame_bytes,
            });
        }
        Ok(messages)
    }

    pub(crate) fn finish(&self) -> Result<(), JsonlError> {
        if self.buffer.is_empty() {
            Ok(())
        } else {
            Err(JsonlError::IncompleteFrame)
        }
    }

    #[allow(
        dead_code,
        reason = "buffer inspection remains available for transport diagnostics"
    )]
    pub(crate) fn buffered_bytes(&self) -> usize {
        self.buffer.len()
    }
}

pub(crate) fn encode_json_line<T: Serialize>(message: &T) -> Result<Vec<u8>, JsonlError> {
    let mut line =
        serde_json::to_vec(message).map_err(|error| JsonlError::InvalidJson(error.to_string()))?;
    line.push(b'\n');
    Ok(line)
}

/// Validate and encode a Pi command. Pi uses a `type` discriminator rather
/// than a JSON-RPC `method`; accepting only object commands catches accidental
/// writes to the sidecar before they can desynchronise the process.
pub(crate) fn encode_command(command: &Value) -> Result<Vec<u8>, JsonlError> {
    let object = command.as_object().ok_or(JsonlError::NonObjectMessage)?;
    let command_type = object
        .get("type")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or(JsonlError::MissingMessageType)?;
    if command_type.contains(['\r', '\n']) {
        return Err(JsonlError::MissingMessageType);
    }
    encode_json_line(command)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PiMessage {
    pub(crate) message_type: String,
    pub(crate) id: Option<String>,
    pub(crate) value: Value,
}

impl PiMessage {
    pub(crate) fn parse(value: Value) -> Result<Self, JsonlError> {
        let object = value.as_object().ok_or(JsonlError::NonObjectMessage)?;
        let message_type = object
            .get("type")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or(JsonlError::MissingMessageType)?
            .to_string();
        let id = object.get("id").and_then(Value::as_str).map(str::to_string);
        Ok(Self {
            message_type,
            id,
            value,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn split_chunks_preserve_jsonl_messages() {
        let mut codec = JsonlCodec::new(1024);
        assert!(codec
            .push(br#"{"type":"message_update"}"#)
            .unwrap()
            .is_empty());
        let messages = codec.push(b"\n{\"type\":\"agent_settled\"}\n").unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["type"], "message_update");
        assert_eq!(messages[1]["type"], "agent_settled");
    }

    pub(crate) fn reject_crlf_and_oversized_frames() {
        let mut codec = JsonlCodec::new(16);
        assert_eq!(
            codec.push(b"{\"type\":\"x\"}\r\n").unwrap_err(),
            JsonlError::CarriageReturnFraming
        );

        let mut codec = JsonlCodec::new(4);
        assert_eq!(
            codec.push(b"12345").unwrap_err(),
            JsonlError::FrameTooLarge { max_bytes: 4 }
        );
    }

    pub(crate) fn command_and_message_validation_use_type_discriminator() {
        let line = encode_command(&json!({ "type": "prompt", "message": "hi" })).unwrap();
        assert_eq!(line.last(), Some(&b'\n'));
        let message = PiMessage::parse(json!({ "id": "r1", "type": "prompt" })).unwrap();
        assert_eq!(message.message_type, "prompt");
        assert_eq!(message.id.as_deref(), Some("r1"));
        assert_eq!(
            encode_command(&json!({ "method": "prompt" })).unwrap_err(),
            JsonlError::MissingMessageType
        );
    }
}

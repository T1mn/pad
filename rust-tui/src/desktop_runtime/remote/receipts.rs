use super::{RemoteCommandOutcome, MAX_REMOTE_FRAME_BYTES};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub(super) fn request_fingerprint(action: &str, params: &Value) -> String {
    let mut digest = Sha256::new();
    digest.update(b"pad.remote.v1 command\0");
    digest_bytes(&mut digest, action.as_bytes());
    digest_json(&mut digest, params);
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn bounded_command_outcome(
    command_id: &str,
    outcome: RemoteCommandOutcome,
) -> RemoteCommandOutcome {
    let frame = json!({
        "type":"command_result",
        "command_id":command_id,
        "ok":outcome.ok,
        "result":&outcome.result,
        "error":&outcome.error,
    });
    if serde_json::to_vec(&frame).map_or(usize::MAX, |encoded| encoded.len())
        <= MAX_REMOTE_FRAME_BYTES
    {
        outcome
    } else {
        RemoteCommandOutcome::rejected(
            "response_too_large",
            "remote response exceeds 1 MiB; request a fresh snapshot",
        )
    }
}

pub(super) fn command_id_conflict() -> RemoteCommandOutcome {
    RemoteCommandOutcome::rejected(
        "command_id_conflict",
        "command_id was already used for a different request",
    )
}

fn digest_json(digest: &mut Sha256, value: &Value) {
    match value {
        Value::Null => digest.update([0]),
        Value::Bool(value) => digest.update([1, u8::from(*value)]),
        Value::Number(value) => {
            digest.update([2]);
            digest_bytes(digest, value.to_string().as_bytes());
        }
        Value::String(value) => {
            digest.update([3]);
            digest_bytes(digest, value.as_bytes());
        }
        Value::Array(values) => {
            digest.update([4]);
            digest.update((values.len() as u64).to_be_bytes());
            for value in values {
                digest_json(digest, value);
            }
        }
        Value::Object(values) => {
            digest.update([5]);
            digest.update((values.len() as u64).to_be_bytes());
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for key in keys {
                digest_bytes(digest, key.as_bytes());
                digest_json(digest, &values[key]);
            }
        }
    }
}

fn digest_bytes(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

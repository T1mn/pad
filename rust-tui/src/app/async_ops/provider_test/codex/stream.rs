pub(super) enum StreamProbe {
    Ok {
        first_output_ms: u64,
        total_ms: u64,
        text: String,
    },
    Failed {
        category: &'static str,
        total_ms: u64,
        message: String,
    },
}

pub(super) async fn read_streaming_response(
    mut response: reqwest::Response,
    started_at: std::time::Instant,
) -> StreamProbe {
    let mut buffer = String::new();
    let mut text = String::new();
    let mut first_output_ms = None;

    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(err) => {
                return StreamProbe::Failed {
                    category: "stream_read",
                    total_ms: super::elapsed_ms(started_at),
                    message: err.to_string(),
                };
            }
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(newline_idx) = buffer.find('\n') {
            let line = buffer[..newline_idx].trim().to_string();
            buffer = buffer[newline_idx + 1..].to_string();
            if !line.starts_with("data:") {
                continue;
            }
            let data = line.trim_start_matches("data:").trim();
            if data == "[DONE]" {
                break;
            }
            let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else {
                continue;
            };
            if let Some(error) = event.get("error") {
                return StreamProbe::Failed {
                    category: "api_error",
                    total_ms: super::elapsed_ms(started_at),
                    message: super::truncate_message(&error.to_string(), 220),
                };
            }
            if event.get("type").and_then(|value| value.as_str()) == Some("response.failed") {
                return StreamProbe::Failed {
                    category: "api_error",
                    total_ms: super::elapsed_ms(started_at),
                    message: super::truncate_message(&event.to_string(), 220),
                };
            }
            if let Some(delta) = event.get("delta").and_then(|value| value.as_str()) {
                if !delta.is_empty() && first_output_ms.is_none() {
                    first_output_ms = Some(super::elapsed_ms(started_at));
                }
                text.push_str(delta);
            }
        }
    }

    match (first_output_ms, text.trim().is_empty()) {
        (Some(first_output_ms), false) => StreamProbe::Ok {
            first_output_ms,
            total_ms: super::elapsed_ms(started_at),
            text: text.trim().to_string(),
        },
        _ => StreamProbe::Failed {
            category: "no_output",
            total_ms: super::elapsed_ms(started_at),
            message: "stream completed without output text delta".to_string(),
        },
    }
}

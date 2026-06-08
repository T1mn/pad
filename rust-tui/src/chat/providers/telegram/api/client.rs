use super::types::TelegramEnvelope;
use crate::log_debug;
use serde::Deserialize;
use std::io;
use std::sync::LazyLock;
use std::time::Instant;

static TELEGRAM_HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .use_rustls_tls()
        .pool_idle_timeout(std::time::Duration::from_secs(30))
        .tcp_keepalive(std::time::Duration::from_secs(30))
        .user_agent(format!("pad/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("telegram http client")
});

pub(super) async fn telegram_api<T: for<'de> Deserialize<'de>>(
    token: &str,
    method: &str,
    payload: &serde_json::Value,
    timeout_secs: u64,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://api.telegram.org/bot{}/{}", token, method);
    let started_at = Instant::now();
    let response = TELEGRAM_HTTP
        .post(url)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .json(payload)
        .send()
        .await
        .map_err(|err| io::Error::other(format!("telegram {} request failed: {}", method, err)))?;

    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|err| io::Error::other(format!("telegram {} body failed: {}", method, err)))?;

    if !status.is_success() {
        return Err(io::Error::other(format!(
            "telegram {} http {}: {}",
            method,
            status,
            String::from_utf8_lossy(&body).trim()
        ))
        .into());
    }

    let envelope: TelegramEnvelope<T> = serde_json::from_slice(&body)?;
    if !envelope.ok {
        return Err(io::Error::other(
            envelope
                .description
                .unwrap_or_else(|| format!("telegram api {} failed", method)),
        )
        .into());
    }
    log_debug!(
        "telegram_api: method={} timeout_s={} elapsed_ms={}",
        method,
        timeout_secs,
        started_at.elapsed().as_millis()
    );
    envelope.result.ok_or_else(|| {
        io::Error::other(format!("telegram api {} returned no result", method)).into()
    })
}

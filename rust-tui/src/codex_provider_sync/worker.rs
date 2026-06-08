use crate::log_debug;
use std::sync::{mpsc, OnceLock};

pub fn enqueue_sync_to_provider(target_provider: String) {
    let target_provider = target_provider.trim().to_string();
    if target_provider.is_empty() {
        return;
    }

    let sender = provider_sync_sender();
    if let Err(err) = sender.send(target_provider) {
        log_debug!(
            "codex_provider_sync: failed to enqueue background sync: {}",
            err
        );
    }
}

fn provider_sync_sender() -> &'static mpsc::Sender<String> {
    static SENDER: OnceLock<mpsc::Sender<String>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name("pad-codex-provider-sync".to_string())
            .spawn(move || provider_sync_worker(rx))
            .expect("spawn provider sync worker");
        tx
    })
}

fn provider_sync_worker(rx: mpsc::Receiver<String>) {
    while let Ok(mut provider) = rx.recv() {
        while let Ok(next_provider) = rx.try_recv() {
            provider = next_provider;
        }
        match super::sync_to_provider(&provider) {
            Ok(result) => {
                log_debug!(
                    "codex_provider_sync: target_provider={} rollout_files={} sqlite_rows={}",
                    provider,
                    result.updated_rollout_files,
                    result.updated_sqlite_rows
                );
            }
            Err(err) => {
                log_debug!(
                    "codex_provider_sync: FAILED target_provider={} err={}",
                    provider,
                    err
                );
            }
        }
    }
}

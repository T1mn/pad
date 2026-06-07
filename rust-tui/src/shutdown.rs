#[cfg(unix)]
pub async fn shutdown_signal() -> &'static str {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    let mut sighup = signal(SignalKind::hangup()).expect("install SIGHUP handler");
    let mut sigpipe = signal(SignalKind::pipe()).expect("install SIGPIPE handler");

    tokio::select! {
        _ = sigint.recv() => "SIGINT",
        _ = sigterm.recv() => "SIGTERM",
        _ = sighup.recv() => "SIGHUP",
        _ = sigpipe.recv() => "SIGPIPE",
    }
}

#[cfg(not(unix))]
pub async fn shutdown_signal() -> &'static str {
    let _ = tokio::signal::ctrl_c().await;
    "CTRL_C"
}

mod listener;
mod process;
mod stream {
    use super::process::process_direct_hook_event;
    use crate::hook::HookEvent;
    use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
    use tokio::net::UnixStream;

    pub(super) async fn handle_direct_hook_stream(
        stream: UnixStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let reader = TokioBufReader::new(stream);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let event: HookEvent = serde_json::from_str(&line)?;
            process_direct_hook_event(&event).await?;
        }

        Ok(())
    }
}

pub(in crate::chat::providers::telegram) use listener::{
    daemon_socket_is_active, start_direct_hook_listener,
};

use crate::runtime_status;

pub(crate) fn pad_is_online() -> bool {
    runtime_status::read_status(&crate::paths::pad_status_path())
        .map(|status| runtime_status::process_alive(status.pid))
        .unwrap_or(false)
}

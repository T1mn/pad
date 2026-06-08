mod db;
mod mutate;
mod path;

#[cfg(test)]
pub(crate) use mutate::mutate_thread_archive_state_at;

pub fn archive_thread(thread_id: &str) -> std::io::Result<()> {
    mutate::mutate_thread_archive_state(thread_id, true)
}

pub fn unarchive_thread(thread_id: &str) -> std::io::Result<()> {
    mutate::mutate_thread_archive_state(thread_id, false)
}

use super::super::SessionReadMode;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub(super) fn for_each_session_line<F>(
    path: &Path,
    read_mode: SessionReadMode,
    mut f: F,
) -> std::io::Result<()>
where
    F: FnMut(&str),
{
    match read_mode {
        SessionReadMode::FullBackfill => {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                f(&line?);
            }
        }
    }

    Ok(())
}

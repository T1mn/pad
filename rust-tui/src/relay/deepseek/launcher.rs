use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub(super) fn write(path: &Path, content: &[u8]) -> io::Result<()> {
    let (temp_path, mut temp_file) = create_temp_file(path)?;
    let write_result = write_private_content(&mut temp_file, content);
    drop(temp_file);

    if let Err(err) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }
    if let Err(err) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }
    Ok(())
}

fn create_temp_file(path: &Path) -> io::Result<(PathBuf, File)> {
    loop {
        let temp_path = temp_path(path)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o700);
        }

        match options.open(&temp_path) {
            Ok(file) => return Ok((temp_path, file)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                let _ = fs::remove_file(&temp_path);
                return Err(err);
            }
        }
    }
}

fn temp_path(path: &Path) -> io::Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "launcher has no parent"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "launcher has no file name"))?;
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".pad-tmp-{}-{id}", std::process::id()));
    Ok(parent.join(temp_name))
}

fn write_private_content(file: &mut File, content: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o700))?;
    }
    file.write_all(content)?;
    file.flush()
}

use std::path::PathBuf;

pub fn sounds_dir() -> PathBuf {
    super::pad_home_dir().join("sounds")
}

pub fn sound_file_path(preset_id: &str) -> PathBuf {
    sounds_dir().join(format!("{preset_id}.wav"))
}

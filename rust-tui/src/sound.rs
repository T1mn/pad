use crate::paths;
use crate::theme::SoundConfig;
use std::fs;
use std::io;

mod catalog;
mod playback;
#[cfg(test)]
mod test_capture;
#[cfg(test)]
mod tests;
mod wav;

pub use catalog::{
    normalize_preset_id, normalize_preset_id_or_default, preset, preset_ids, presets, SoundEvent,
};
#[cfg(test)]
pub(crate) use test_capture::{take_test_playbacks, with_test_sound_capture, TestPlayback};

pub fn play_event(config: &SoundConfig, event: SoundEvent) -> io::Result<bool> {
    let event_config = event.config(config);
    if !config.enabled || !event_config.enabled {
        return Ok(false);
    }

    play_internal(Some(event), &event_config.preset)
}

pub fn preview_preset(preset_id: &str) -> io::Result<bool> {
    play_internal(None, preset_id)
}

pub fn ensure_runtime_assets() -> io::Result<()> {
    fs::create_dir_all(paths::sounds_dir())?;

    for preset in presets() {
        let path = paths::sound_file_path(preset.id);
        let desired = wav::render_wav_bytes(preset);
        if fs::read(&path).ok().as_deref() != Some(desired.as_slice()) {
            fs::write(&path, &desired)?;
        }
    }

    Ok(())
}

#[cfg(test)]
fn play_internal(event: Option<SoundEvent>, preset_id: &str) -> io::Result<bool> {
    let Some(preset_id) = normalize_preset_id(preset_id) else {
        return Ok(false);
    };

    if test_capture::should_capture_test_sounds() {
        test_capture::record_test_playback(event, preset_id);
        return Ok(true);
    }

    Ok(false)
}

#[cfg(not(test))]
fn play_internal(_event: Option<SoundEvent>, preset_id: &str) -> io::Result<bool> {
    let Some(preset_id) = normalize_preset_id(preset_id) else {
        return Ok(false);
    };

    if sounds_disabled() {
        return Ok(false);
    }

    let path = paths::sound_file_path(preset_id);
    if !path.exists() {
        ensure_runtime_assets()?;
    }
    let path = paths::sound_file_path(preset_id);
    if !path.exists() {
        return Ok(false);
    }

    #[cfg(target_os = "macos")]
    {
        let Some(spec) = playback::macos_command_spec(&path, playback::command_exists) else {
            return Ok(false);
        };
        playback::spawn_audio(&spec.program, &spec.args)?;
        Ok(true)
    }

    #[cfg(target_os = "linux")]
    {
        let Some(spec) = playback::linux_command_spec(&path, playback::command_exists) else {
            return Ok(false);
        };
        playback::spawn_audio(&spec.program, &spec.args)?;
        Ok(true)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        Ok(false)
    }
}

#[cfg(not(test))]
fn sounds_disabled() -> bool {
    cfg!(test) || std::env::var_os("PAD_DISABLE_SOUNDS").is_some()
}

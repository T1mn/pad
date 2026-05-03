use std::process::Command;

const WIDTH_LEVELS: &[u16] = &[35, 50, 65];
const WIDTH_OPTION: &str = "@pad_sider_width_percent";

pub fn default_width() -> &'static str {
    "50%"
}

pub fn stored_or_default_width(target_pane: &str) -> String {
    show_option(target_pane, WIDTH_OPTION)
        .filter(|value| WIDTH_LEVELS.contains(&parse_percent(value).unwrap_or_default()))
        .unwrap_or_else(|| default_width().to_string())
}

pub fn resize_from_helper(target_pane: Option<&str>, wider: bool) -> Result<(), String> {
    let helper_pane = std::env::var("TMUX_PANE").map_err(|_| "TMUX_PANE is missing".to_string())?;
    let current = target_pane
        .and_then(stored_width)
        .or_else(|| current_width_percent(&helper_pane).map(nearest_width_level))
        .unwrap_or(50);
    let next = next_width_level(current, wider);
    let cols = width_columns(&helper_pane, next)?;
    run_tmux(&["resize-pane", "-t", &helper_pane, "-x", &cols.to_string()])?;
    if let Some(target_pane) = target_pane {
        let _ = run_tmux(&[
            "set-option",
            "-p",
            "-t",
            target_pane,
            WIDTH_OPTION,
            &format!("{next}%"),
        ]);
    }
    Ok(())
}

pub fn next_width_level(current: u16, wider: bool) -> u16 {
    if wider {
        WIDTH_LEVELS
            .iter()
            .copied()
            .find(|level| *level > current)
            .unwrap_or(*WIDTH_LEVELS.last().unwrap_or(&50))
    } else {
        WIDTH_LEVELS
            .iter()
            .rev()
            .copied()
            .find(|level| *level < current)
            .unwrap_or(WIDTH_LEVELS[0])
    }
}

fn nearest_width_level(current: u16) -> u16 {
    WIDTH_LEVELS
        .iter()
        .copied()
        .min_by_key(|level| level.abs_diff(current))
        .unwrap_or(50)
}

fn current_width_percent(pane: &str) -> Option<u16> {
    let output = run_tmux(&[
        "display-message",
        "-p",
        "-t",
        pane,
        "#{pane_width} #{window_width}",
    ])
    .ok()?;
    let mut parts = output.split_whitespace();
    let pane_width = parts.next()?.parse::<u16>().ok()?;
    let window_width = parts.next()?.parse::<u16>().ok()?;
    if window_width == 0 {
        return None;
    }
    Some(((pane_width as u32 * 100) / window_width as u32) as u16)
}

fn width_columns(pane: &str, percent: u16) -> Result<u16, String> {
    let output = run_tmux(&["display-message", "-p", "-t", pane, "#{window_width}"])?;
    let window_width = output
        .trim()
        .parse::<u16>()
        .map_err(|err| format!("invalid window width: {err}"))?;
    Ok(((window_width as u32 * percent as u32) / 100).max(20) as u16)
}

fn show_option(target: &str, key: &str) -> Option<String> {
    run_tmux(&["show-options", "-p", "-v", "-t", target, key])
        .ok()
        .map(|value| value.trim().to_string())
}

fn stored_width(target_pane: &str) -> Option<u16> {
    show_option(target_pane, WIDTH_OPTION).and_then(|value| parse_percent(&value))
}

fn parse_percent(value: &str) -> Option<u16> {
    value.trim().trim_end_matches('%').parse::<u16>().ok()
}

fn run_tmux(args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|err| format!("tmux {}: {err}", args.join(" ")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "tmux {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{default_width, nearest_width_level, next_width_level};

    #[test]
    fn default_width_is_half() {
        assert_eq!(default_width(), "50%");
    }

    #[test]
    fn width_levels_step_without_wrapping() {
        assert_eq!(next_width_level(50, true), 65);
        assert_eq!(next_width_level(65, true), 65);
        assert_eq!(next_width_level(50, false), 35);
        assert_eq!(next_width_level(35, false), 35);
    }

    #[test]
    fn nearest_width_level_handles_tmux_rounding() {
        assert_eq!(nearest_width_level(49), 50);
        assert_eq!(nearest_width_level(51), 50);
        assert_eq!(nearest_width_level(64), 65);
    }
}

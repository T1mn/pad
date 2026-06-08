pub(in crate::app::actions) fn normalize_prompt(text: &str) -> Result<String, &'static str> {
    let prompt = text.trim();
    if prompt.is_empty() {
        Err("Clipboard is empty")
    } else {
        Ok(prompt.to_string())
    }
}

pub(in crate::app::actions) fn prompt_preview(prompt: &str) -> &str {
    prompt
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(prompt)
}

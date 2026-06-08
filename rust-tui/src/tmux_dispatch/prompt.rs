use std::error::Error;

mod literal;
mod paste;
mod util;

pub fn dispatch_prompt(pane_id: &str, prompt: &str) -> Result<(), Box<dyn Error>> {
    if util::is_multiline(prompt) {
        return paste::dispatch_pasted_prompt(pane_id, prompt);
    }

    match literal::dispatch_literal_prompt(pane_id, prompt) {
        Ok(()) => Ok(()),
        Err(err) => {
            log_debug!(
                "tmux_dispatch: literal send failed pane={} len={} err={}, falling back to paste",
                pane_id,
                prompt.chars().count(),
                err
            );
            paste::dispatch_pasted_prompt(pane_id, prompt)
        }
    }
}

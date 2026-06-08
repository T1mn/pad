use super::super::*;

pub(super) async fn handle_use_command(
    config: &Config,
    state: &mut TelegramState,
    chat_id: &str,
    arg: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let idx = arg.parse::<usize>().ok().and_then(|n| n.checked_sub(1));
    let Some(idx) = idx else {
        send_text(&config.telegram.bot_token, chat_id, tg(locale, "use.usage")).await?;
        return Ok(());
    };
    let Some(entry) = state.agent_snapshot.get(idx).cloned() else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "use.invalid"),
        )
        .await?;
        return Ok(());
    };
    let panels = live_panels().map_err(telegram_error)?;
    if !panels.iter().any(|panel| panel.pane_id == entry.pane_id) {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "pane.stale"),
        )
        .await?;
        return Ok(());
    }
    state.selected_target = Some(SelectedTarget {
        pane_id: entry.pane_id.clone(),
        label: entry.label.clone(),
    });
    send_text(
        &config.telegram.bot_token,
        chat_id,
        &tg_fmt(locale, "target.switched", entry.label),
    )
    .await?;
    Ok(())
}

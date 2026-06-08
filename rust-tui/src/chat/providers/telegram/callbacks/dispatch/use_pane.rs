use super::super::super::*;

pub(super) async fn handle_use_pane_callback(
    config: &Config,
    state: &mut TelegramState,
    query_id: &str,
    chat_id: &str,
    pane_id: &str,
    locale: crate::i18n::Locale,
) -> TelegramResult<()> {
    let panels = live_panels().map_err(telegram_error)?;
    if let Some(panel) = panels.iter().find(|panel| panel.pane_id == pane_id) {
        let selected = SelectedTarget {
            pane_id: panel.pane_id.clone(),
            label: format_agent_line_for_button(panel, locale),
        };
        state.selected_target = Some(selected.clone());
        answer_callback_query(
            &config.telegram.bot_token,
            query_id,
            Some(tg(locale, "callback.switched")),
        )
        .await?;
        send_text(
            &config.telegram.bot_token,
            chat_id,
            &tg_fmt(locale, "target.switched", selected.label),
        )
        .await?;
    } else {
        answer_callback_query(
            &config.telegram.bot_token,
            query_id,
            Some(tg(locale, "callback.stale")),
        )
        .await?;
    }
    Ok(())
}

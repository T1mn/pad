use super::super::state::TelegramState;
use super::body::{help_codex_body, help_overview_body, help_workflow_body};
use super::labels::{help_action_label, help_button_label, help_page_title};
use super::page::HelpPage;
use super::text::help_text;
use serde_json::json;

pub(in crate::chat::providers::telegram) fn help_message_payload(
    locale: crate::i18n::Locale,
    state: &TelegramState,
    chat_id: serde_json::Value,
    message_id: Option<i64>,
    page: HelpPage,
) -> serde_json::Value {
    let mut payload = json!({
        "chat_id": chat_id,
        "text": help_page_html(locale, state, page),
        "parse_mode": "HTML",
        "disable_web_page_preview": true,
        "reply_markup": {
            "inline_keyboard": build_help_keyboard(locale, page)
        }
    });
    if let Some(message_id) = message_id {
        payload["message_id"] = serde_json::Value::from(message_id);
    }
    payload
}

pub(in crate::chat::providers::telegram) fn build_help_keyboard(
    locale: crate::i18n::Locale,
    page: HelpPage,
) -> Vec<Vec<serde_json::Value>> {
    let nav_button = |target: HelpPage| {
        let text = if target == page {
            format!("• {}", help_button_label(locale, target))
        } else {
            help_button_label(locale, target).to_string()
        };
        json!({
            "text": text,
            "callback_data": target.callback_data(),
        })
    };
    vec![
        vec![
            nav_button(HelpPage::Overview),
            nav_button(HelpPage::Codex),
            nav_button(HelpPage::Workflow),
        ],
        vec![
            json!({
                "text": help_action_label(locale, "list"),
                "callback_data": "help:list",
            }),
            json!({
                "text": help_action_label(locale, "padstatus"),
                "callback_data": "help:padstatus",
            }),
        ],
    ]
}

pub(in crate::chat::providers::telegram) fn help_page_html(
    locale: crate::i18n::Locale,
    state: &TelegramState,
    page: HelpPage,
) -> String {
    let target = state
        .selected_target
        .as_ref()
        .map(|target| html_escape(&target.label))
        .unwrap_or_else(|| help_text(locale, "target.none").to_string());
    let page_title = help_page_title(locale, page);
    let page_body = match page {
        HelpPage::Overview => help_overview_body(locale),
        HelpPage::Codex => help_codex_body(locale),
        HelpPage::Workflow => help_workflow_body(locale),
    };
    format!(
        "<b>{}</b>\n<blockquote>{}: <code>{}</code></blockquote>\n\n<b>{}</b>\n{}",
        help_text(locale, "title"),
        help_text(locale, "target"),
        target,
        page_title,
        page_body
    )
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

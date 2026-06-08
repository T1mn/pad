use super::client::telegram_api;
use super::TELEGRAM_INTERACTIVE_TIMEOUT_SECS;
use crate::chat::providers::telegram::locale::tg;
use serde::Serialize;
use serde_json::json;

#[derive(Clone, Debug, Serialize)]
struct TelegramCommandSpec {
    command: &'static str,
    description: String,
}

pub(in crate::chat::providers::telegram) async fn set_my_commands(
    token: &str,
    locale: crate::i18n::Locale,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let commands = vec![
        TelegramCommandSpec {
            command: "start",
            description: tg(locale, "command.start").to_string(),
        },
        TelegramCommandSpec {
            command: "help",
            description: tg(locale, "command.help").to_string(),
        },
        TelegramCommandSpec {
            command: "list",
            description: tg(locale, "command.list").to_string(),
        },
        TelegramCommandSpec {
            command: "use",
            description: tg(locale, "command.use").to_string(),
        },
        TelegramCommandSpec {
            command: "history",
            description: tg(locale, "command.history").to_string(),
        },
        TelegramCommandSpec {
            command: "diag",
            description: tg(locale, "command.diag").to_string(),
        },
        TelegramCommandSpec {
            command: "restart",
            description: tg(locale, "command.restart").to_string(),
        },
        TelegramCommandSpec {
            command: "status",
            description: tg(locale, "command.status").to_string(),
        },
        TelegramCommandSpec {
            command: "fast",
            description: tg(locale, "command.fast").to_string(),
        },
        TelegramCommandSpec {
            command: "compact",
            description: tg(locale, "command.compact").to_string(),
        },
        TelegramCommandSpec {
            command: "reset",
            description: tg(locale, "command.reset").to_string(),
        },
        TelegramCommandSpec {
            command: "stop",
            description: tg(locale, "command.stop").to_string(),
        },
    ];
    let _: serde_json::Value = telegram_api(
        token,
        "setMyCommands",
        &json!({
            "commands": commands
        }),
        TELEGRAM_INTERACTIVE_TIMEOUT_SECS,
    )
    .await?;
    Ok(())
}

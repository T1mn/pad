mod format;
mod select;
mod text;

pub(super) use format::{tg_fmt, tg_fmt2, tg_fmt3};
pub(super) use select::{locale_prefers_chinese, telegram_locale};
pub(super) use text::tg;

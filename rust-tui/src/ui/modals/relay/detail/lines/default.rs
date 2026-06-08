use crate::i18n::Locale;
use crate::theme::{ProviderConfig, Theme};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(super) fn default_detail_lines(
    provider: &ProviderConfig,
    theme: &Theme,
    locale: Locale,
    make_val: &impl Fn(usize, &str) -> String,
    field_style: &impl Fn(usize) -> Style,
    masked_api_key: String,
) -> Vec<Line<'static>> {
    vec![
        super::detail_line(theme, locale, "relay.label"),
        Line::from(Span::styled(make_val(0, &provider.label), field_style(0))),
        Line::from(""),
        super::detail_line(theme, locale, "relay.base_url"),
        Line::from(Span::styled(
            make_val(1, &provider.base_url),
            field_style(1),
        )),
        Line::from(""),
        super::detail_line(theme, locale, "relay.api_key"),
        Line::from(Span::styled(masked_api_key, field_style(2))),
    ]
}

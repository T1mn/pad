use std::collections::HashMap;
use std::sync::LazyLock;

macro_rules! locale_map {
    ($name:ident, $( $key:expr => $val:expr ),+ $(,)?) => {
        pub(super) static $name: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
            HashMap::from([ $( ($key, $val), )+ ])
        });
    };
}

mod de;
mod en;
mod fr;
mod ja;
mod zh_cn;
mod zh_tw;

mod locale;
pub use locale::Locale;

pub fn t(locale: Locale, key: &str) -> &str {
    match locale {
        Locale::ZhCN => zh_cn::ZH_CN.get(key).copied().unwrap_or(key),
        Locale::ZhTW => zh_tw::ZH_TW.get(key).copied().unwrap_or(key),
        Locale::En => en::EN.get(key).copied().unwrap_or(key),
        Locale::Ja => ja::JA.get(key).copied().unwrap_or(key),
        Locale::De => de::DE.get(key).copied().unwrap_or(key),
        Locale::Fr => fr::FR.get(key).copied().unwrap_or(key),
    }
}

#[cfg(test)]
mod tests;

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

mod locale {
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum Locale {
        ZhCN,
        ZhTW,
        En,
        Ja,
        De,
        Fr,
    }

    impl Locale {
        pub fn from_str(s: &str) -> Self {
            match s {
                "zh-cn" | "zh_CN" => Locale::ZhCN,
                "zh-tw" | "zh_TW" => Locale::ZhTW,
                "en" | "en-us" | "en_US" => Locale::En,
                "ja" => Locale::Ja,
                "de" => Locale::De,
                "fr" => Locale::Fr,
                _ => Locale::En,
            }
        }

        pub fn as_str(&self) -> &'static str {
            match self {
                Locale::ZhCN => "zh-cn",
                Locale::ZhTW => "zh-tw",
                Locale::En => "en",
                Locale::Ja => "ja",
                Locale::De => "de",
                Locale::Fr => "fr",
            }
        }

        pub fn display_name(&self) -> &'static str {
            match self {
                Locale::ZhCN => "简体中文",
                Locale::ZhTW => "繁體中文",
                Locale::En => "English",
                Locale::Ja => "日本語",
                Locale::De => "Deutsch",
                Locale::Fr => "Français",
            }
        }
    }
}
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
pub(crate) mod tests;

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

    #[allow(dead_code)]
    pub fn next(&self) -> Self {
        match self {
            Locale::ZhCN => Locale::ZhTW,
            Locale::ZhTW => Locale::En,
            Locale::En => Locale::Ja,
            Locale::Ja => Locale::De,
            Locale::De => Locale::Fr,
            Locale::Fr => Locale::ZhCN,
        }
    }
}

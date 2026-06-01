//! UI language selection and locale handling.

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[allow(non_camel_case_types)]
pub enum Language {
    #[serde(rename = "en")]
    English,

    #[serde(rename = "zh-tw")]
    TChinese,

    #[serde(rename = "zh-cn")]
    SChinese,

    #[serde(rename = "vi")]
    Vietnamese,

    #[serde(rename = "id")]
    Indonesian,

    #[serde(rename = "es")]
    Spanish,

    #[serde(rename = "pt-br")]
    BPortuguese,

    #[serde(rename = "fil")]
    Filipino,
}

impl Default for Language {
    fn default() -> Self {
        let locale = sys_locale::get_locale().as_deref().unwrap_or("en").to_lowercase();
        if locale.contains("zh-hk") || locale.contains("zh-tw") || locale.contains("zh-hant") {
            Self::TChinese
        } else if locale.contains("zh") {
            Self::SChinese
        } else if locale.starts_with("vi") {
            Self::Vietnamese
        } else if locale.starts_with("id") {
            Self::Indonesian
        } else if locale.starts_with("es") {
            Self::Spanish
        } else if locale.starts_with("pt-br") {
            Self::BPortuguese
        } else if locale.starts_with("fil") {
            Self::Filipino
        } else {
            Self::English
        }
    }
}

impl Language {
    pub const CHOICES: &[(Self, &'static str)] = &[
        Self::English.choice(),
        Self::TChinese.choice(),
        Self::SChinese.choice(),
        Self::Vietnamese.choice(),
        Self::Indonesian.choice(),
        Self::Spanish.choice(),
        Self::BPortuguese.choice(),
        Self::Filipino.choice(),
    ];

    pub fn set_locale(&self) {
        rust_i18n::set_locale(self.locale_str());
    }

    pub const fn locale_str(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::TChinese => "zh-tw",
            Language::SChinese => "zh-cn",
            Language::Vietnamese => "vi",
            Language::Indonesian => "id",
            Language::Spanish => "es",
            Language::BPortuguese => "pt-br",
            Language::Filipino => "fil",
        }
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::TChinese => "繁體中文",
            Language::SChinese => "简体中文",
            Language::Vietnamese => "Tiếng Việt",
            Language::Indonesian => "Bahasa Indonesia",
            Language::Spanish => "Español (ES)",
            Language::BPortuguese => "Português (Brasil)",
            Language::Filipino => "Filipino",
        }
    }

    pub const fn choice(self) -> (Self, &'static str) {
        (self, self.name())
    }
}

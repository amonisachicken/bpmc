//! Tiny bilingual string table.
//!
//! The display language is picked from the locale environment: if any of
//! `LC_ALL` / `LC_MESSAGES` / `LANG` contains `zh` (case-insensitive), the UI
//! is Chinese, otherwise English. This works on Linux, macOS and Windows
//! (where `LANG` is usually unset, so English is the fallback).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
}

impl Lang {
    /// First locale-looking environment variable across platforms.
    pub fn from_env() -> Self {
        Self::from_locale(
            std::env::var("LC_ALL")
                .ok()
                .filter(|v| !v.is_empty())
                .or_else(|| std::env::var("LC_MESSAGES").ok().filter(|v| !v.is_empty()))
                .or_else(|| std::env::var("LANG").ok().filter(|v| !v.is_empty()))
                .unwrap_or_default()
                .as_str(),
        )
    }

    /// Classify a locale string such as `zh_CN.UTF-8` or `en_US.UTF-8`.
    pub fn from_locale(locale: &str) -> Self {
        if locale.to_lowercase().contains("zh") {
            Lang::Zh
        } else {
            Lang::En
        }
    }

    /// All user-visible text for this language.
    pub fn text(self) -> &'static Text {
        match self {
            Lang::En => &EN,
            Lang::Zh => &ZH,
        }
    }
}

/// Every string the UI shows, in one language.
#[derive(Debug, Clone, Copy)]
pub struct Text {
    pub title: &'static str,
    pub measuring: &'static str,
    pub idle: &'static str,
    pub bpm_unit: &'static str,
    pub hint_tap: &'static str,
    pub hint_quit: &'static str,
    pub hint_reset: &'static str,
    pub taps: &'static str,
    pub last_interval: &'static str,
    pub steady: &'static str,
    pub history: &'static str,
    pub none: &'static str,
    pub hint_short: &'static str,
    pub reset_done: &'static str,
    pub terminal_too_small: &'static str,
}

pub static EN: Text = Text {
    title: "BPM Counter",
    measuring: "tap space to measure",
    idle: "idle - waiting for taps",
    bpm_unit: "BPM",
    hint_tap: "Space  tap",
    hint_quit: "q / Esc  quit",
    hint_reset: "r  reset",
    taps: "Taps",
    last_interval: "Last interval",
    steady: "Jitter",
    history: "Intervals",
    none: "-",
    hint_short: "Ready.",
    reset_done: "Reset.",
    terminal_too_small: "Terminal too small - please enlarge the window",
};

pub static ZH: Text = Text {
    title: "BPM 计数器",
    measuring: "按空格键开始测量",
    idle: "空闲 - 等待按键",
    bpm_unit: "BPM",
    hint_tap: "空格  计数",
    hint_quit: "q / Esc  退出",
    hint_reset: "r  重置",
    taps: "按键次数",
    last_interval: "最近间隔",
    steady: "抖动",
    history: "间隔历史",
    none: "-",
    hint_short: "就绪。",
    reset_done: "已重置。",
    terminal_too_small: "终端窗口太小 - 请放大窗口",
};

impl Lang {
    pub fn t(self) -> &'static Text {
        self.text()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_locales_are_detected() {
        assert_eq!(Lang::from_locale("zh_CN.UTF-8"), Lang::Zh);
        assert_eq!(Lang::from_locale("zh_TW"), Lang::Zh);
        assert_eq!(Lang::from_locale("ZH_cn"), Lang::Zh);
        assert_eq!(
            Lang::from_locale("Chinese (Simplified)_China.936"),
            Lang::En
        );
    }

    #[test]
    fn other_locales_fall_back_to_english() {
        assert_eq!(Lang::from_locale("en_US.UTF-8"), Lang::En);
        assert_eq!(Lang::from_locale("C"), Lang::En);
        assert_eq!(Lang::from_locale(""), Lang::En);
        assert_eq!(Lang::from_locale("de_DE.UTF-8"), Lang::En);
    }

    #[test]
    fn both_languages_have_text() {
        assert!(!Lang::En.text().title.is_empty());
        assert!(!Lang::Zh.text().title.is_empty());
    }
}

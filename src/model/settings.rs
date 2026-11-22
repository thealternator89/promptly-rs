use serde::{Deserialize, Serialize};

/// User-selected application theme preference persisted in `settings.json`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    #[default]
    Auto,
    Light,
    Dark,
}

impl ThemePreference {
    /// Advances to the next mode in the cycle: Auto -> Light -> Dark -> Auto.
    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::Auto,
        }
    }

    /// Tooltip describing the current theme preference and the next click target.
    pub fn tooltip(self) -> &'static str {
        match self {
            Self::Auto => "Theme: Auto (follows system). Click to switch to Light.",
            Self::Light => "Theme: Light. Click to switch to Dark.",
            Self::Dark => "Theme: Dark. Click to switch to Auto.",
        }
    }
}

/// Global application settings persisted in `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub disallowed_domains: String,
    #[serde(default)]
    pub theme_mode: ThemePreference,
}

impl AppSettings {
    /// Returns normalized, non-empty domain entries from `disallowed_domains`.
    pub fn disallowed_domain_list(&self) -> Vec<String> {
        self.disallowed_domains
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_lowercase)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_preference_defaults_to_auto_and_cycles_in_order() {
        assert_eq!(ThemePreference::default(), ThemePreference::Auto);
        assert_eq!(ThemePreference::Auto.next(), ThemePreference::Light);
        assert_eq!(ThemePreference::Light.next(), ThemePreference::Dark);
        assert_eq!(ThemePreference::Dark.next(), ThemePreference::Auto);
    }

    #[test]
    fn legacy_settings_json_without_theme_mode_defaults_to_auto() {
        let parsed: AppSettings =
            serde_json::from_str(r#"{"disallowedDomains":"corp.local"}"#).expect("parse legacy");
        assert_eq!(parsed.disallowed_domains, "corp.local");
        assert_eq!(parsed.theme_mode, ThemePreference::Auto);
    }

    #[test]
    fn theme_mode_round_trips_with_camel_case_names() {
        for (pref, expected_str) in [
            (ThemePreference::Auto, "auto"),
            (ThemePreference::Light, "light"),
            (ThemePreference::Dark, "dark"),
        ] {
            let settings = AppSettings {
                disallowed_domains: "example.com".into(),
                theme_mode: pref,
            };
            let value = serde_json::to_value(&settings).expect("serialize settings");
            assert_eq!(value["themeMode"], expected_str);

            let deserialized: AppSettings =
                serde_json::from_value(value).expect("deserialize settings");
            assert_eq!(deserialized, settings);
        }
    }
}

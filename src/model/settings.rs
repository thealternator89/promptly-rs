use serde::{Deserialize, Serialize};

/// Global application settings persisted in `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub disallowed_domains: String,
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

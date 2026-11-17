use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use uuid::Uuid;

use crate::model::{AppSettings, Prompt};

/// Summary of a migration run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MigrationReport {
    pub performed: bool,
    pub prompts_copied: usize,
    pub settings_copied: bool,
    pub order_copied: bool,
}

/// Filesystem persistence for prompts, custom ordering, settings, and legacy Electron migration.
#[derive(Debug, Clone)]
pub struct StorageService {
    data_dir: PathBuf,
    legacy_dirs: Vec<PathBuf>,
}

impl StorageService {
    /// Creates a `StorageService` using the OS application data directory (`Promptly-RS`)
    /// and standard legacy Electron directories (`Promptly` / `promptly`).
    pub fn new() -> Result<Self> {
        let base_dir = dirs::config_dir()
            .context("Could not resolve OS application configuration directory")?;
        let data_dir = base_dir.join("Promptly-RS");
        let legacy_dirs = vec![base_dir.join("Promptly"), base_dir.join("promptly")];
        Ok(Self {
            data_dir,
            legacy_dirs,
        })
    }

    /// Creates a `StorageService` with explicit directories (useful for tests or custom paths).
    pub fn with_dirs(data_dir: impl Into<PathBuf>, legacy_dirs: Vec<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            legacy_dirs,
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn legacy_dirs(&self) -> &[PathBuf] {
        &self.legacy_dirs
    }

    pub fn detected_legacy_dir(&self) -> Option<&Path> {
        self.legacy_dirs
            .iter()
            .find(|p| p.is_dir())
            .map(PathBuf::as_path)
    }

    pub fn prompts_dir(&self) -> PathBuf {
        self.data_dir.join("prompts")
    }

    pub fn order_file(&self) -> PathBuf {
        self.prompts_dir().join("order.json")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    fn migrated_marker(&self) -> PathBuf {
        self.data_dir.join(".migrated")
    }

    /// Checks whether the data directory has already been initialized.
    pub fn is_initialized(&self) -> bool {
        self.migrated_marker().exists() || self.prompts_dir().exists()
    }

    /// Initializes the data directory on first run, non-destructively copying data from
    /// the first detected legacy Electron `Promptly` directory if uninitialized.
    pub fn init_and_migrate(&self) -> Result<MigrationReport> {
        if self.is_initialized() {
            return Ok(MigrationReport::default());
        }

        let mut report = MigrationReport::default();
        if let Some(legacy_dir) = self.legacy_dirs.iter().find(|p| p.is_dir()) {
            report = self.migrate_from_directory(legacy_dir)?;
        } else {
            fs::create_dir_all(self.prompts_dir())?;
        }

        fs::create_dir_all(&self.data_dir)?;
        fs::write(self.migrated_marker(), b"1")?;
        Ok(report)
    }

    /// Non-destructively copies `settings.json`, `prompts/order.json`, and `prompts/*.json`
    /// from `source_dir` into this `StorageService`'s data directory.
    pub fn migrate_from_directory(&self, source_dir: &Path) -> Result<MigrationReport> {
        if !source_dir.is_dir() {
            bail!(
                "Migration source directory does not exist: {}",
                source_dir.display()
            );
        }

        fs::create_dir_all(self.prompts_dir())?;

        let mut report = MigrationReport {
            performed: true,
            ..Default::default()
        };

        let src_settings = source_dir.join("settings.json");
        if src_settings.is_file() {
            fs::copy(&src_settings, self.settings_file())
                .context("Failed to copy settings.json during migration")?;
            report.settings_copied = true;
        }

        let src_prompts_dir = source_dir.join("prompts");
        if src_prompts_dir.is_dir() {
            let src_order = src_prompts_dir.join("order.json");
            if src_order.is_file() {
                fs::copy(&src_order, self.order_file())
                    .context("Failed to copy prompts/order.json during migration")?;
                report.order_copied = true;
            }

            for entry in fs::read_dir(&src_prompts_dir)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if file_name == "order.json" || !file_name.ends_with(".json") {
                    continue;
                }
                fs::copy(&path, self.prompts_dir().join(file_name))
                    .with_context(|| format!("Failed to copy prompt file {file_name}"))?;
                report.prompts_copied += 1;
            }
        }

        if !self.migrated_marker().exists() {
            fs::write(self.migrated_marker(), b"1")?;
        }

        Ok(report)
    }

    /// Loads all `prompts/*.json` (excluding `order.json`) and sorts them by `order.json`
    /// priority, falling back to `created_at` descending for unordered items.
    pub fn load_prompts(&self) -> Result<Vec<Prompt>> {
        let prompts_dir = self.prompts_dir();
        if !prompts_dir.is_dir() {
            return Ok(Vec::new());
        }

        let order = self.load_prompts_order()?;
        let order_index: HashMap<String, usize> = order
            .into_iter()
            .enumerate()
            .map(|(idx, id)| (id, idx))
            .collect();

        let mut prompts = Vec::new();
        for entry in fs::read_dir(&prompts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if file_name == "order.json" || !file_name.ends_with(".json") {
                continue;
            }

            let bytes = fs::read(&path)
                .with_context(|| format!("Failed to read prompt file {}", path.display()))?;
            let prompt: Prompt = serde_json::from_slice(&bytes)
                .with_context(|| format!("Failed to parse prompt file {}", path.display()))?;
            prompts.push(prompt);
        }

        prompts.sort_by(|a, b| {
            match (order_index.get(&a.id), order_index.get(&b.id)) {
                (Some(idx_a), Some(idx_b)) => idx_a.cmp(idx_b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => b.created_at.cmp(&a.created_at),
            }
        });

        Ok(prompts)
    }

    /// Atomically writes a prompt to `prompts/<id>.json`.
    pub fn save_prompt(&self, prompt: &Prompt) -> Result<()> {
        validate_prompt_id(&prompt.id)?;
        let path = self.prompts_dir().join(format!("{}.json", prompt.id));
        let bytes = serde_json::to_vec_pretty(prompt)?;
        atomic_write(&path, &bytes)
    }

    /// Deletes `prompts/<id>.json` and removes `id` from `prompts/order.json` if present.
    pub fn delete_prompt(&self, id: &str) -> Result<()> {
        validate_prompt_id(id)?;
        let path = self.prompts_dir().join(format!("{id}.json"));
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove prompt file {}", path.display()))?;
        }

        let order_path = self.order_file();
        if order_path.is_file() {
            let order = self.load_prompts_order()?;
            let filtered: Vec<String> = order.into_iter().filter(|item| item != id).collect();
            self.save_prompts_order(&filtered)?;
        }

        Ok(())
    }

    /// Loads `prompts/order.json`, or returns an empty `Vec` if it does not exist.
    pub fn load_prompts_order(&self) -> Result<Vec<String>> {
        let path = self.order_file();
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&path)?;
        let order: Vec<String> = serde_json::from_slice(&bytes)?;
        Ok(order)
    }

    /// Atomically writes `prompts/order.json`.
    pub fn save_prompts_order(&self, ids: &[String]) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(ids)?;
        atomic_write(&self.order_file(), &bytes)
    }

    /// Loads `settings.json`, returning `AppSettings::default()` if the file does not exist.
    pub fn load_settings(&self) -> Result<AppSettings> {
        let path = self.settings_file();
        if !path.is_file() {
            return Ok(AppSettings::default());
        }
        let bytes = fs::read(&path)?;
        let settings: AppSettings = serde_json::from_slice(&bytes)?;
        Ok(settings)
    }

    /// Atomically writes `settings.json`.
    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(settings)?;
        atomic_write(&self.settings_file(), &bytes)
    }
}

fn validate_prompt_id(id: &str) -> Result<()> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        bail!("Invalid prompt id: '{id}'");
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("Target path has no parent directory")?;
    fs::create_dir_all(parent)?;

    let tmp_name = format!(".tmp-{}", Uuid::new_v4());
    let tmp_path = parent.join(tmp_name);
    fs::write(&tmp_path, bytes)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PromptPart;
    use tempfile::tempdir;

    fn sample_prompt(id: &str, title: &str, created_at: u64) -> Prompt {
        Prompt {
            id: id.to_string(),
            title: title.to_string(),
            description: String::new(),
            created_at,
            parts: vec![PromptPart::Fixed {
                id: "f1".into(),
                text: "Hello".into(),
            }],
        }
    }

    #[test]
    fn legacy_migration_copies_on_first_run_and_does_not_overwrite() {
        let temp = tempdir().expect("tempdir");
        let legacy_dir = temp.path().join("Promptly");
        let legacy_prompts = legacy_dir.join("prompts");
        fs::create_dir_all(&legacy_prompts).unwrap();

        let p1 = sample_prompt("p1", "Legacy Prompt", 100);
        fs::write(
            legacy_prompts.join("p1.json"),
            serde_json::to_vec_pretty(&p1).unwrap(),
        )
        .unwrap();
        fs::write(legacy_prompts.join("order.json"), br#"["p1"]"#).unwrap();
        fs::write(
            legacy_dir.join("settings.json"),
            br#"{"disallowedDomains":"corp.local"}"#,
        )
        .unwrap();

        let new_dir = temp.path().join("Promptly-RS");
        let storage = StorageService::with_dirs(&new_dir, vec![legacy_dir.clone()]);

        let first_report = storage.init_and_migrate().expect("first init");
        assert!(first_report.performed);
        assert_eq!(first_report.prompts_copied, 1);
        assert!(first_report.settings_copied);
        assert!(first_report.order_copied);

        // Legacy files remain intact
        assert!(legacy_prompts.join("p1.json").is_file());

        // Modify new settings and verify subsequent init_and_migrate does NOT overwrite
        let updated_settings = AppSettings {
            disallowed_domains: "new.example.com".into(),
        };
        storage.save_settings(&updated_settings).unwrap();

        let second_report = storage.init_and_migrate().expect("second init");
        assert!(!second_report.performed);
        assert_eq!(storage.load_settings().unwrap(), updated_settings);
    }

    #[test]
    fn prompt_sorting_matches_order_json_and_created_at_fallback() {
        let temp = tempdir().expect("tempdir");
        let storage = StorageService::with_dirs(temp.path().join("data"), vec![]);

        let p_old = sample_prompt("old", "Old Unordered", 100);
        let p_new = sample_prompt("new", "New Unordered", 500);
        let p_ord_2 = sample_prompt("ord2", "Ordered Second", 50);
        let p_ord_1 = sample_prompt("ord1", "Ordered First", 10);

        for p in [&p_old, &p_new, &p_ord_2, &p_ord_1] {
            storage.save_prompt(p).unwrap();
        }
        storage
            .save_prompts_order(&["ord1".into(), "ord2".into()])
            .unwrap();

        let loaded = storage.load_prompts().unwrap();
        let ids: Vec<&str> = loaded.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["ord1", "ord2", "new", "old"]);
    }

    #[test]
    fn delete_prompt_removes_file_and_order_entry() {
        let temp = tempdir().expect("tempdir");
        let storage = StorageService::with_dirs(temp.path().join("data"), vec![]);

        storage.save_prompt(&sample_prompt("a", "A", 1)).unwrap();
        storage.save_prompt(&sample_prompt("b", "B", 2)).unwrap();
        storage
            .save_prompts_order(&["a".into(), "b".into()])
            .unwrap();

        storage.delete_prompt("a").unwrap();

        assert!(!storage.prompts_dir().join("a.json").exists());
        assert!(storage.prompts_dir().join("b.json").exists());
        assert_eq!(storage.load_prompts_order().unwrap(), vec!["b"]);
    }
}

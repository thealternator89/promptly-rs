use std::collections::HashSet;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Supported syntax languages for `Code` blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Csharp,
    Typescript,
    Javascript,
    Python,
    Sql,
    Json,
    Bash,
    Powershell,
    Html,
    Xml,
    Yaml,
    Markdown,
    Diff,
}

impl Language {
    pub const ALL: &'static [Language] = &[
        Language::Csharp,
        Language::Typescript,
        Language::Javascript,
        Language::Python,
        Language::Sql,
        Language::Json,
        Language::Bash,
        Language::Powershell,
        Language::Html,
        Language::Xml,
        Language::Yaml,
        Language::Markdown,
        Language::Diff,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Csharp => "csharp",
            Language::Typescript => "typescript",
            Language::Javascript => "javascript",
            Language::Python => "python",
            Language::Sql => "sql",
            Language::Json => "json",
            Language::Bash => "bash",
            Language::Powershell => "powershell",
            Language::Html => "html",
            Language::Xml => "xml",
            Language::Yaml => "yaml",
            Language::Markdown => "markdown",
            Language::Diff => "diff",
        }
    }
}

fn default_level() -> u8 {
    1
}

/// A single modular block in a Promptly template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PromptPart {
    Fixed {
        id: String,
        #[serde(default)]
        text: String,
    },
    Heading {
        id: String,
        #[serde(default)]
        text: String,
        #[serde(default = "default_level")]
        level: u8,
        #[serde(
            rename = "excludeIfNextEmpty",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        exclude_if_next_empty: Option<bool>,
    },
    Custom {
        id: String,
        #[serde(rename = "defaultText", default)]
        default_text: Option<String>,
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(rename = "singleLine", default)]
        single_line: Option<bool>,
    },
    Quote {
        id: String,
    },
    Code {
        id: String,
        #[serde(default)]
        language: Option<String>,
    },
    Hr {
        id: String,
    },
    Repeatable {
        id: String,
        #[serde(rename = "templateParts", default)]
        template_parts: Vec<PromptPart>,
    },
}

impl PromptPart {
    pub fn id(&self) -> &str {
        match self {
            PromptPart::Fixed { id, .. }
            | PromptPart::Heading { id, .. }
            | PromptPart::Custom { id, .. }
            | PromptPart::Quote { id }
            | PromptPart::Code { id, .. }
            | PromptPart::Hr { id }
            | PromptPart::Repeatable { id, .. } => id,
        }
    }
}

/// A saved Promptly template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub created_at: u64,
    pub parts: Vec<PromptPart>,
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn non_empty_string_opt(val: Option<&Value>) -> Option<String> {
    val.and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn normalize_part(raw: &Value) -> Result<PromptPart> {
    let obj = raw
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Each part must be a JSON object"))?;

    let part_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Part is missing a valid string 'type'"))?;

    let id = non_empty_string_opt(obj.get("id")).unwrap_or_else(|| Uuid::new_v4().to_string());

    match part_type {
        "fixed" => {
            let text = obj
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Ok(PromptPart::Fixed { id, text })
        }
        "heading" => {
            let text = obj
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let level = obj
                .get("level")
                .and_then(|v| v.as_u64())
                .map(|n| n.clamp(1, 3) as u8)
                .unwrap_or_else(default_level);
            let exclude_if_next_empty = obj
                .get("excludeIfNextEmpty")
                .and_then(|v| v.as_bool())
                .filter(|b| *b);
            Ok(PromptPart::Heading {
                id,
                text,
                level,
                exclude_if_next_empty,
            })
        }
        "custom" => {
            let default_text = obj
                .get("defaultText")
                .and_then(|v| v.as_str())
                .map(ToString::to_string);
            let placeholder = obj
                .get("placeholder")
                .and_then(|v| v.as_str())
                .map(ToString::to_string);
            let single_line = obj.get("singleLine").and_then(|v| v.as_bool());
            Ok(PromptPart::Custom {
                id,
                default_text,
                placeholder,
                single_line,
            })
        }
        "quote" => Ok(PromptPart::Quote { id }),
        "code" => {
            let language = obj
                .get("language")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(ToString::to_string);
            Ok(PromptPart::Code { id, language })
        }
        "hr" => Ok(PromptPart::Hr { id }),
        "repeatable" => {
            let raw_children = obj
                .get("templateParts")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    anyhow::anyhow!("Repeatable part must contain a 'templateParts' array")
                })?;
            let mut template_parts = Vec::with_capacity(raw_children.len());
            for child in raw_children {
                template_parts.push(normalize_part(child)?);
            }
            Ok(PromptPart::Repeatable { id, template_parts })
        }
        other => bail!("Unsupported part type: '{other}'"),
    }
}

/// Parses raw JSON, validates required fields and part types, generates missing IDs,
/// and resolves prompt ID collisions against `existing_ids`.
pub fn import_and_normalize_prompt(
    raw_json: &str,
    existing_ids: &HashSet<String>,
) -> Result<Prompt> {
    let root: Value =
        serde_json::from_str(raw_json).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))?;

    let obj = root
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Prompt JSON must be an object"))?;

    let title = obj
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Prompt must have a non-empty 'title'"))?
        .to_string();

    let raw_parts = obj
        .get("parts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Prompt must have a 'parts' array"))?;

    let mut parts = Vec::with_capacity(raw_parts.len());
    for raw_part in raw_parts {
        parts.push(normalize_part(raw_part)?);
    }

    let candidate_id = non_empty_string_opt(obj.get("id"));
    let id = match candidate_id {
        Some(existing) if !existing_ids.contains(&existing) => existing,
        _ => Uuid::new_v4().to_string(),
    };

    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let created_at = obj
        .get("createdAt")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(current_timestamp_ms);

    Ok(Prompt {
        id,
        title,
        description,
        created_at,
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_real_prompt_json() {
        let json = r#"{
            "id": "prompt-123",
            "title": "Code Review",
            "description": "Structured PR review template",
            "createdAt": 1700000000000,
            "parts": [
                {
                    "id": "h1",
                    "type": "heading",
                    "text": "Context",
                    "level": 2,
                    "excludeIfNextEmpty": true
                },
                {
                    "id": "f1",
                    "type": "fixed",
                    "text": "Please review the following changes."
                },
                {
                    "id": "c1",
                    "type": "custom",
                    "defaultText": "N/A",
                    "placeholder": "Enter summary",
                    "singleLine": true
                },
                {
                    "id": "q1",
                    "type": "quote"
                },
                {
                    "id": "code1",
                    "type": "code",
                    "language": "rust"
                },
                {
                    "id": "hr1",
                    "type": "hr"
                },
                {
                    "id": "rep1",
                    "type": "repeatable",
                    "templateParts": [
                        {
                            "id": "rh1",
                            "type": "heading",
                            "text": "File",
                            "level": 3
                        },
                        {
                            "id": "rc1",
                            "type": "code",
                            "language": "diff"
                        }
                    ]
                }
            ]
        }"#;

        let prompt: Prompt = serde_json::from_str(json).expect("deserialize");
        assert_eq!(prompt.id, "prompt-123");
        assert_eq!(prompt.parts.len(), 7);

        let serialized = serde_json::to_string_pretty(&prompt).expect("serialize");
        let reparsed: Prompt = serde_json::from_str(&serialized).expect("re-deserialize");
        assert_eq!(prompt, reparsed);
    }

    #[test]
    fn import_generates_missing_ids_and_resolves_collision() {
        let raw = r#"{
            "id": "existing-id",
            "title": "Imported Prompt",
            "parts": [
                { "type": "heading", "text": "Intro" },
                { "type": "custom", "singleLine": false },
                {
                    "type": "repeatable",
                    "templateParts": [
                        { "type": "quote" }
                    ]
                }
            ]
        }"#;

        let mut existing = HashSet::new();
        existing.insert("existing-id".to_string());

        let prompt = import_and_normalize_prompt(raw, &existing).expect("should import");
        assert_ne!(prompt.id, "existing-id");
        assert!(!prompt.id.is_empty());
        assert_eq!(prompt.title, "Imported Prompt");
        assert_eq!(prompt.parts.len(), 3);
        for part in &prompt.parts {
            assert!(!part.id().is_empty());
            if let PromptPart::Repeatable { template_parts, .. } = part {
                assert_eq!(template_parts.len(), 1);
                assert!(!template_parts[0].id().is_empty());
            }
        }
    }

    #[test]
    fn import_rejects_missing_title_or_invalid_part_type() {
        let existing = HashSet::new();

        let no_title = r#"{ "title": "   ", "parts": [] }"#;
        assert!(import_and_normalize_prompt(no_title, &existing).is_err());

        let bad_part = r#"{
            "title": "Valid",
            "parts": [{ "type": "unknown_block" }]
        }"#;
        assert!(import_and_normalize_prompt(bad_part, &existing).is_err());

        let bad_nested = r#"{
            "title": "Valid",
            "parts": [{
                "type": "repeatable",
                "templateParts": [{ "type": "bogus" }]
            }]
        }"#;
        assert!(import_and_normalize_prompt(bad_nested, &existing).is_err());
    }
}

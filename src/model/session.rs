use std::collections::HashMap;

use crate::model::{Prompt, PromptPart};

/// Cached runtime state for a Prompt Viewer or Scratchpad session.
///
/// Full interactive block state and compilation wiring are expanded in Phase 5.
#[derive(Debug, Clone)]
pub struct ViewerSession {
    pub prompt_id: String,
    pub title: String,
    pub session_parts: Vec<PromptPart>,
    pub values: HashMap<String, String>,
    pub languages: HashMap<String, String>,
    pub raw_values: HashMap<String, String>,
    pub repeatable_instances: HashMap<String, Vec<String>>,
}

impl ViewerSession {
    pub fn scratchpad() -> Self {
        Self {
            prompt_id: "scratchpad".to_string(),
            title: "Scratchpad".to_string(),
            session_parts: Vec::new(),
            values: HashMap::new(),
            languages: HashMap::new(),
            raw_values: HashMap::new(),
            repeatable_instances: HashMap::new(),
        }
    }

    pub fn from_prompt(prompt: &Prompt) -> Self {
        let mut values = HashMap::new();
        let mut languages = HashMap::new();
        let mut repeatable_instances = HashMap::new();

        for part in &prompt.parts {
            match part {
                PromptPart::Custom {
                    id, default_text, ..
                } => {
                    if let Some(default) = default_text {
                        values.insert(id.clone(), default.clone());
                    }
                }
                PromptPart::Code { id, language } => {
                    if let Some(lang) = language {
                        languages.insert(id.clone(), lang.clone());
                    }
                }
                PromptPart::Repeatable { id, template_parts } => {
                    let instance_id = "inst-1".to_string();
                    for child in template_parts {
                        let key = format!("{id}_{instance_id}_{}", child.id());
                        match child {
                            PromptPart::Custom { default_text, .. } => {
                                if let Some(default) = default_text {
                                    values.insert(key, default.clone());
                                }
                            }
                            PromptPart::Code { language, .. } => {
                                if let Some(lang) = language {
                                    languages.insert(key, lang.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    repeatable_instances.insert(id.clone(), vec![instance_id]);
                }
                _ => {}
            }
        }

        Self {
            prompt_id: prompt.id.clone(),
            title: prompt.title.clone(),
            session_parts: prompt.parts.clone(),
            values,
            languages,
            raw_values: HashMap::new(),
            repeatable_instances,
        }
    }
}

use std::collections::HashMap;

use crate::model::PromptPart;

/// Builds the composite key for a child part inside a repeatable instance.
pub fn composite_key(parent_id: &str, instance_id: &str, child_id: &str) -> String {
    format!("{parent_id}_{instance_id}_{child_id}")
}

/// Compiles a list of `PromptPart`s into a final Markdown string using a two-pass algorithm:
/// 1. Evaluates each block (including nested repeatable instances).
/// 2. Applies conditional heading exclusion (`excludeIfNextEmpty`).
/// 3. Filters out empty blocks and joins with `"\n\n"`.
pub fn compile_prompt(
    parts: &[PromptPart],
    values: &HashMap<String, String>,
    languages: &HashMap<String, String>,
    repeatable_instances: &HashMap<String, Vec<String>>,
) -> String {
    compile_parts(parts, None, values, languages, repeatable_instances)
}

fn compile_parts(
    parts: &[PromptPart],
    parent_instance: Option<(&str, &str)>,
    values: &HashMap<String, String>,
    languages: &HashMap<String, String>,
    repeatable_instances: &HashMap<String, Vec<String>>,
) -> String {
    // Pass 1: evaluate each block
    let mut evaluated: Vec<String> = parts
        .iter()
        .map(|part| evaluate_part(part, parent_instance, values, languages, repeatable_instances))
        .collect();

    // Pass 2: conditional heading exclusion
    for (i, part) in parts.iter().enumerate() {
        if let PromptPart::Heading {
            exclude_if_next_empty: Some(true),
            ..
        } = part
        {
            let next_is_empty = evaluated
                .get(i + 1)
                .is_none_or(|next| next.trim().is_empty());
            if next_is_empty {
                evaluated[i].clear();
            }
        }
    }

    // Join non-empty blocks
    evaluated
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn evaluate_part(
    part: &PromptPart,
    parent_instance: Option<(&str, &str)>,
    values: &HashMap<String, String>,
    languages: &HashMap<String, String>,
    repeatable_instances: &HashMap<String, Vec<String>>,
) -> String {
    let key = match parent_instance {
        Some((parent_id, instance_id)) => composite_key(parent_id, instance_id, part.id()),
        None => part.id().to_string(),
    };

    match part {
        PromptPart::Heading { text, level, .. } => {
            let hashes = "#".repeat((*level).clamp(1, 3) as usize);
            format!("{hashes} {text}")
        }
        PromptPart::Fixed { text, .. } => text.clone(),
        PromptPart::Custom { .. } => values.get(&key).cloned().unwrap_or_default(),
        PromptPart::Quote { .. } => {
            let val = values.get(&key).map(String::as_str).unwrap_or("");
            if val.trim().is_empty() {
                String::new()
            } else {
                val.split('\n')
                    .map(|line| format!("> {line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        PromptPart::Code { language, .. } => {
            let val = values.get(&key).map(String::as_str).unwrap_or("");
            if val.trim().is_empty() {
                String::new()
            } else {
                let lang = languages
                    .get(&key)
                    .map(String::as_str)
                    .or(language.as_deref())
                    .unwrap_or("");
                format!("```{lang}\n{val}\n```")
            }
        }
        PromptPart::Hr { .. } => "---".to_string(),
        PromptPart::Repeatable { id, template_parts } => {
            let Some(instances) = repeatable_instances.get(id) else {
                return String::new();
            };

            instances
                .iter()
                .map(|instance_id| {
                    compile_parts(
                        template_parts,
                        Some((id.as_str(), instance_id.as_str())),
                        values,
                        languages,
                        repeatable_instances,
                    )
                })
                .filter(|s| !s.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_all_top_level_block_types() {
        let parts = vec![
            PromptPart::Heading {
                id: "h1".into(),
                text: "Task".into(),
                level: 1,
                exclude_if_next_empty: None,
            },
            PromptPart::Fixed {
                id: "f1".into(),
                text: "Follow these steps:".into(),
            },
            PromptPart::Custom {
                id: "c1".into(),
                default_text: None,
                placeholder: None,
                single_line: Some(true),
            },
            PromptPart::Quote { id: "q1".into() },
            PromptPart::Code {
                id: "code1".into(),
                language: Some("rust".into()),
            },
            PromptPart::Hr { id: "hr1".into() },
        ];

        let mut values = HashMap::new();
        values.insert("c1".into(), "Refactor parser".into());
        values.insert("q1".into(), "Line one\nLine two".into());
        values.insert("code1".into(), "fn main() {}".into());

        let out = compile_prompt(&parts, &values, &HashMap::new(), &HashMap::new());
        let expected = "\
# Task

Follow these steps:

Refactor parser

> Line one
> Line two

```rust
fn main() {}
```

---";
        assert_eq!(out, expected);
    }

    #[test]
    fn conditional_heading_excluded_when_next_is_empty_or_missing() {
        let parts = vec![
            PromptPart::Heading {
                id: "h1".into(),
                text: "Notes".into(),
                level: 2,
                exclude_if_next_empty: Some(true),
            },
            PromptPart::Custom {
                id: "c1".into(),
                default_text: None,
                placeholder: None,
                single_line: None,
            },
            PromptPart::Heading {
                id: "h2".into(),
                text: "Trailing".into(),
                level: 2,
                exclude_if_next_empty: Some(true),
            },
        ];

        let mut values = HashMap::new();
        values.insert("c1".into(), "   ".into());

        let out = compile_prompt(&parts, &values, &HashMap::new(), &HashMap::new());
        assert_eq!(out, "");

        values.insert("c1".into(), "Some note".into());
        let out_with_note = compile_prompt(&parts, &values, &HashMap::new(), &HashMap::new());
        assert_eq!(out_with_note, "## Notes\n\nSome note");
    }

    #[test]
    fn repeatable_instances_and_nested_conditional_headings() {
        let parts = vec![PromptPart::Repeatable {
            id: "rep".into(),
            template_parts: vec![
                PromptPart::Heading {
                    id: "rh".into(),
                    text: "Snippet".into(),
                    level: 3,
                    exclude_if_next_empty: Some(true),
                },
                PromptPart::Code {
                    id: "rc".into(),
                    language: Some("python".into()),
                },
            ],
        }];

        let mut repeatable_instances = HashMap::new();
        repeatable_instances.insert("rep".into(), vec!["inst1".into(), "inst2".into()]);

        let mut values = HashMap::new();
        // inst1 has code, inst2 is empty (so its heading is also excluded)
        values.insert(composite_key("rep", "inst1", "rc"), "print(1)".into());

        let mut languages = HashMap::new();
        languages.insert(composite_key("rep", "inst1", "rc"), "bash".into());

        let out = compile_prompt(&parts, &values, &languages, &repeatable_instances);
        assert_eq!(out, "### Snippet\n\n```bash\nprint(1)\n```");
    }
}

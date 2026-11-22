pub mod code;
pub mod custom;
pub mod fixed;
pub mod frame;
pub mod heading;
pub mod hr;
pub mod quote;
pub mod repeatable;

pub use frame::{
    BlockFrame, BlockPreviewSummary, DragBlockPreview, first_non_empty_line,
    trailing_drop_zone_style,
};

/// Renders a `Textarea` wrapped so pressing `Tab` (`IndentInline`) inserts
/// four spaces instead of moving focus to the next control.
pub fn tab_textarea(
    textarea: &Entity<gpui_kit::component::input::TextareaState>,
    configure: impl FnOnce(gpui_kit::component::input::Textarea) -> gpui_kit::component::input::Textarea,
) -> AnyElement {
    use gpui_kit::component::input::{IndentInline, Textarea};

    let state = textarea.clone();
    div()
        .w_full()
        .capture_action(move |_: &IndentInline, window, cx| {
            cx.stop_propagation();
            state.update(cx, |s, cx| {
                EntityInputHandler::replace_text_in_range(s, None, "    ", window, cx);
            });
        })
        .child(configure(Textarea::new(textarea)))
        .into_any_element()
}

use std::collections::HashMap;

use gpui_kit::*;
use uuid::Uuid;

use crate::model::{PromptPart, ViewerSession};
use crate::services::compiler::composite_key;

use self::code::CodeEditorState;
use self::custom::CustomEditorState;
use self::fixed::FixedEditorState;
use self::heading::HeadingEditorState;

/// The 7 modular block types supported by Promptly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Heading,
    Fixed,
    Custom,
    Quote,
    Code,
    Hr,
    Repeatable,
}

impl BlockKind {
    pub const ALL: &'static [BlockKind] = &[
        BlockKind::Heading,
        BlockKind::Fixed,
        BlockKind::Custom,
        BlockKind::Quote,
        BlockKind::Code,
        BlockKind::Hr,
        BlockKind::Repeatable,
    ];

    pub const CHILD_KINDS: &'static [BlockKind] = &[
        BlockKind::Heading,
        BlockKind::Fixed,
        BlockKind::Custom,
        BlockKind::Quote,
        BlockKind::Code,
        BlockKind::Hr,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            BlockKind::Heading => "Heading",
            BlockKind::Fixed => "Fixed Text",
            BlockKind::Custom => "Custom Input",
            BlockKind::Quote => "Quote",
            BlockKind::Code => "Code Block",
            BlockKind::Hr => "Rule",
            BlockKind::Repeatable => "Repeatable",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            BlockKind::Heading => "HEADING",
            BlockKind::Fixed => "FIXED",
            BlockKind::Custom => "CUSTOM",
            BlockKind::Quote => "QUOTE",
            BlockKind::Code => "CODE",
            BlockKind::Hr => "HR",
            BlockKind::Repeatable => "REPEATABLE",
        }
    }
}

/// Retained GPUI state for a single block inside the Template Editor.
pub enum EditorPartState {
    Heading(HeadingEditorState),
    Fixed(FixedEditorState),
    Custom(CustomEditorState),
    Quote {
        id: String,
    },
    Code(CodeEditorState),
    Hr {
        id: String,
    },
    Repeatable {
        id: String,
        template_parts: Vec<EditorPartState>,
    },
}

impl EditorPartState {
    pub fn id(&self) -> &str {
        match self {
            EditorPartState::Heading(s) => &s.id,
            EditorPartState::Fixed(s) => &s.id,
            EditorPartState::Custom(s) => &s.id,
            EditorPartState::Quote { id } => id,
            EditorPartState::Code(s) => &s.id,
            EditorPartState::Hr { id } => id,
            EditorPartState::Repeatable { id, .. } => id,
        }
    }

    pub fn kind(&self) -> BlockKind {
        match self {
            EditorPartState::Heading(_) => BlockKind::Heading,
            EditorPartState::Fixed(_) => BlockKind::Fixed,
            EditorPartState::Custom(_) => BlockKind::Custom,
            EditorPartState::Quote { .. } => BlockKind::Quote,
            EditorPartState::Code(_) => BlockKind::Code,
            EditorPartState::Hr { .. } => BlockKind::Hr,
            EditorPartState::Repeatable { .. } => BlockKind::Repeatable,
        }
    }

    pub fn badge(&self) -> &'static str {
        self.kind().badge()
    }

    /// Builds a compact single-line summary for collapsed drag-and-drop cards in the Editor.
    pub fn preview_summary(&self, cx: &App) -> BlockPreviewSummary {
        match self {
            EditorPartState::Heading(s) => {
                let text = s.text_input.read(cx).value();
                match first_non_empty_line(&text) {
                    Some(line) => BlockPreviewSummary::text(format!("H{} · {line}", s.level)),
                    None => BlockPreviewSummary::muted(format!("H{} · Empty heading", s.level)),
                }
            }
            EditorPartState::Fixed(s) => {
                let text = s.text_input.read(cx).value();
                match first_non_empty_line(&text) {
                    Some(line) => BlockPreviewSummary::text(line),
                    None => BlockPreviewSummary::muted("Empty fixed text"),
                }
            }
            EditorPartState::Custom(s) => {
                let def = s.default_text_input.read(cx).value();
                if let Some(line) = first_non_empty_line(&def) {
                    return BlockPreviewSummary::text(line);
                }
                let ph = s.placeholder_input.read(cx).value();
                if let Some(line) = first_non_empty_line(&ph) {
                    return BlockPreviewSummary::muted(line);
                }
                BlockPreviewSummary::muted(if s.single_line {
                    "Single-line input"
                } else {
                    "Multi-line input"
                })
            }
            EditorPartState::Quote { .. } => {
                BlockPreviewSummary::muted("User-fillable blockquote")
            }
            EditorPartState::Code(s) => match s.selected_language(cx) {
                Some(lang) => BlockPreviewSummary::text(lang),
                None => BlockPreviewSummary::muted("Plain code block"),
            },
            EditorPartState::Hr { .. } => BlockPreviewSummary::hr(),
            EditorPartState::Repeatable { template_parts, .. } => {
                let n = template_parts.len();
                if n == 0 {
                    return BlockPreviewSummary::muted("Empty repeatable");
                }
                let count_label = if n == 1 {
                    "1 part".to_string()
                } else {
                    format!("{n} parts")
                };
                if let Some(first_meaningful) = template_parts.iter().find_map(|child| {
                    let s = child.preview_summary(cx);
                    (!s.is_muted && !s.is_hr).then_some(s.text)
                }) {
                    BlockPreviewSummary::text(format!("{count_label} · {first_meaningful}"))
                } else {
                    let kinds = template_parts
                        .iter()
                        .map(|p| p.kind().label())
                        .collect::<Vec<_>>()
                        .join(", ");
                    BlockPreviewSummary::muted(format!("{count_label} · {kinds}"))
                }
            }
        }
    }

    pub fn new_blank(kind: BlockKind, window: &mut Window, cx: &mut App) -> Self {
        let id = Uuid::new_v4().to_string();
        match kind {
            BlockKind::Heading => {
                EditorPartState::Heading(HeadingEditorState::new(id, "", 2, false, window, cx))
            }
            BlockKind::Fixed => {
                EditorPartState::Fixed(FixedEditorState::new(id, "", window, cx))
            }
            BlockKind::Custom => EditorPartState::Custom(CustomEditorState::new(
                id, None, None, false, window, cx,
            )),
            BlockKind::Quote => EditorPartState::Quote { id },
            BlockKind::Code => {
                EditorPartState::Code(CodeEditorState::new(id, None, window, cx))
            }
            BlockKind::Hr => EditorPartState::Hr { id },
            BlockKind::Repeatable => EditorPartState::Repeatable {
                id,
                template_parts: Vec::new(),
            },
        }
    }

    pub fn from_prompt_part(part: &PromptPart, window: &mut Window, cx: &mut App) -> Self {
        match part {
            PromptPart::Fixed { id, text } => {
                EditorPartState::Fixed(FixedEditorState::new(id.clone(), text, window, cx))
            }
            PromptPart::Heading {
                id,
                text,
                level,
                exclude_if_next_empty,
            } => EditorPartState::Heading(HeadingEditorState::new(
                id.clone(),
                text,
                *level,
                exclude_if_next_empty.unwrap_or(false),
                window,
                cx,
            )),
            PromptPart::Custom {
                id,
                default_text,
                placeholder,
                single_line,
            } => EditorPartState::Custom(CustomEditorState::new(
                id.clone(),
                default_text.as_deref(),
                placeholder.as_deref(),
                single_line.unwrap_or(false),
                window,
                cx,
            )),
            PromptPart::Quote { id } => EditorPartState::Quote { id: id.clone() },
            PromptPart::Code { id, language } => EditorPartState::Code(CodeEditorState::new(
                id.clone(),
                language.as_deref(),
                window,
                cx,
            )),
            PromptPart::Hr { id } => EditorPartState::Hr { id: id.clone() },
            PromptPart::Repeatable { id, template_parts } => EditorPartState::Repeatable {
                id: id.clone(),
                template_parts: template_parts
                    .iter()
                    .map(|p| EditorPartState::from_prompt_part(p, window, cx))
                    .collect(),
            },
        }
    }

    pub fn to_prompt_part(&self, cx: &App) -> PromptPart {
        match self {
            EditorPartState::Fixed(s) => PromptPart::Fixed {
                id: s.id.clone(),
                text: s.text_input.read(cx).value().to_string(),
            },
            EditorPartState::Heading(s) => PromptPart::Heading {
                id: s.id.clone(),
                text: s.text_input.read(cx).value().to_string(),
                level: s.level,
                exclude_if_next_empty: if s.exclude_if_next_empty {
                    Some(true)
                } else {
                    None
                },
            },
            EditorPartState::Custom(s) => {
                let def = s.default_text_input.read(cx).value().to_string();
                let ph = s.placeholder_input.read(cx).value().to_string();
                PromptPart::Custom {
                    id: s.id.clone(),
                    default_text: if def.is_empty() { None } else { Some(def) },
                    placeholder: if ph.is_empty() { None } else { Some(ph) },
                    single_line: Some(s.single_line),
                }
            }
            EditorPartState::Quote { id } => PromptPart::Quote { id: id.clone() },
            EditorPartState::Code(s) => PromptPart::Code {
                id: s.id.clone(),
                language: s.selected_language(cx),
            },
            EditorPartState::Hr { id } => PromptPart::Hr { id: id.clone() },
            EditorPartState::Repeatable { id, template_parts } => PromptPart::Repeatable {
                id: id.clone(),
                template_parts: template_parts
                    .iter()
                    .map(|p| p.to_prompt_part(cx))
                    .collect(),
            },
        }
    }
}

/// Builds a compact single-line summary for collapsed drag-and-drop cards in the Viewer/Scratchpad.
pub fn viewer_part_preview_summary(
    part: &PromptPart,
    session: &ViewerSession,
) -> BlockPreviewSummary {
    viewer_part_preview_summary_from_maps(
        part,
        &session.values,
        &session.languages,
        &session.repeatable_instances,
    )
}

fn part_kind_label(part: &PromptPart) -> &'static str {
    match part {
        PromptPart::Heading { .. } => BlockKind::Heading.label(),
        PromptPart::Fixed { .. } => BlockKind::Fixed.label(),
        PromptPart::Custom { .. } => BlockKind::Custom.label(),
        PromptPart::Quote { .. } => BlockKind::Quote.label(),
        PromptPart::Code { .. } => BlockKind::Code.label(),
        PromptPart::Hr { .. } => BlockKind::Hr.label(),
        PromptPart::Repeatable { .. } => BlockKind::Repeatable.label(),
    }
}

fn repeatable_child_snippet(
    part: &PromptPart,
    key: Option<&str>,
    values: &HashMap<String, String>,
    languages: &HashMap<String, String>,
) -> Option<String> {
    match part {
        PromptPart::Heading { text, level, .. } => {
            first_non_empty_line(text).map(|line| format!("H{level} · {line}"))
        }
        PromptPart::Fixed { text, .. } => first_non_empty_line(text),
        PromptPart::Custom { .. } | PromptPart::Quote { .. } => {
            key.and_then(|k| values.get(k)).and_then(|v| first_non_empty_line(v))
        }
        PromptPart::Code { language, .. } => {
            let line = key
                .and_then(|k| values.get(k))
                .and_then(|v| first_non_empty_line(v));
            let lang = key
                .and_then(|k| languages.get(k))
                .map(String::as_str)
                .or(language.as_deref())
                .map(str::trim)
                .filter(|l| !l.is_empty());
            match (lang, line) {
                (Some(l), Some(code)) => Some(format!("{l} · {code}")),
                (None, Some(code)) => Some(code),
                _ => None,
            }
        }
        PromptPart::Hr { .. } | PromptPart::Repeatable { .. } => None,
    }
}

pub(crate) fn viewer_part_preview_summary_from_maps(
    part: &PromptPart,
    values: &HashMap<String, String>,
    languages: &HashMap<String, String>,
    repeatable_instances: &HashMap<String, Vec<String>>,
) -> BlockPreviewSummary {
    match part {
        PromptPart::Heading { text, level, .. } => match first_non_empty_line(text) {
            Some(line) => BlockPreviewSummary::text(format!("H{level} · {line}")),
            None => BlockPreviewSummary::muted(format!("H{level} · Empty heading")),
        },
        PromptPart::Fixed { text, .. } => match first_non_empty_line(text) {
            Some(line) => BlockPreviewSummary::text(line),
            None => BlockPreviewSummary::muted("Empty fixed text"),
        },
        PromptPart::Custom {
            id, placeholder, ..
        } => {
            if let Some(line) = values.get(id).and_then(|v| first_non_empty_line(v)) {
                return BlockPreviewSummary::text(line);
            }
            if let Some(line) = placeholder.as_deref().and_then(first_non_empty_line) {
                return BlockPreviewSummary::muted(line);
            }
            BlockPreviewSummary::muted("Empty input")
        }
        PromptPart::Quote { id } => {
            match values.get(id).and_then(|v| first_non_empty_line(v)) {
                Some(line) => BlockPreviewSummary::text(line),
                None => BlockPreviewSummary::muted("Empty quote"),
            }
        }
        PromptPart::Code { id, language } => {
            let effective_lang = languages
                .get(id)
                .map(String::as_str)
                .or(language.as_deref())
                .map(str::trim)
                .filter(|l| !l.is_empty());
            let code_line = values.get(id).and_then(|v| first_non_empty_line(v));
            match (effective_lang, code_line) {
                (Some(lang), Some(line)) => BlockPreviewSummary::text(format!("{lang} · {line}")),
                (None, Some(line)) => BlockPreviewSummary::text(line),
                (Some(lang), None) => {
                    BlockPreviewSummary::muted(format!("{lang} · Empty code block"))
                }
                (None, None) => BlockPreviewSummary::muted("Empty code block"),
            }
        }
        PromptPart::Hr { .. } => BlockPreviewSummary::hr(),
        PromptPart::Repeatable { id, template_parts } => {
            let instances = repeatable_instances
                .get(id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let inst_count = instances.len();
            let inst_label = if inst_count == 1 {
                "1 instance".to_string()
            } else {
                format!("{inst_count} instances")
            };

            if template_parts.is_empty() {
                return BlockPreviewSummary::muted(format!("{inst_label} · Empty repeatable"));
            }

            let first_inst_id = instances.first().map(String::as_str);
            if let Some(snippet) = template_parts.iter().find_map(|child| {
                let key = first_inst_id.map(|iid| composite_key(id, iid, child.id()));
                repeatable_child_snippet(child, key.as_deref(), values, languages)
            }) {
                BlockPreviewSummary::text(format!("{inst_label} · {snippet}"))
            } else {
                let part_label = if template_parts.len() == 1 {
                    part_kind_label(&template_parts[0]).to_string()
                } else {
                    format!("{} parts", template_parts.len())
                };
                BlockPreviewSummary::muted(format!("{inst_label} · {part_label}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        BlockPreviewSummary, first_non_empty_line, viewer_part_preview_summary_from_maps,
    };
    use crate::model::PromptPart;
    use crate::services::compiler::composite_key;

    #[test]
    fn first_non_empty_line_handles_blank_lines_crlf_and_tabs() {
        assert_eq!(first_non_empty_line(""), None);
        assert_eq!(first_non_empty_line("   \n\t\r\n  "), None);
        assert_eq!(
            first_non_empty_line("\n  \n\tFirst\tline\r\nSecond line"),
            Some("First line".to_string())
        );
        assert_eq!(
            first_non_empty_line("Single line"),
            Some("Single line".to_string())
        );
    }

    #[test]
    fn viewer_summary_covers_all_part_types_and_fallbacks() {
        let mut values = HashMap::new();
        let mut languages = HashMap::new();
        let mut instances = HashMap::new();

        // Heading
        let heading = PromptPart::Heading {
            id: "h1".into(),
            text: "\n  Context  \nMore".into(),
            level: 2,
            exclude_if_next_empty: None,
        };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&heading, &values, &languages, &instances),
            BlockPreviewSummary::text("H2 · Context")
        );
        let empty_heading = PromptPart::Heading {
            id: "h2".into(),
            text: "   ".into(),
            level: 1,
            exclude_if_next_empty: None,
        };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&empty_heading, &values, &languages, &instances),
            BlockPreviewSummary::muted("H1 · Empty heading")
        );

        // Fixed
        let fixed = PromptPart::Fixed {
            id: "f1".into(),
            text: "\n\nPlease review.\nDetails".into(),
        };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&fixed, &values, &languages, &instances),
            BlockPreviewSummary::text("Please review.")
        );
        let empty_fixed = PromptPart::Fixed {
            id: "f2".into(),
            text: "".into(),
        };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&empty_fixed, &values, &languages, &instances),
            BlockPreviewSummary::muted("Empty fixed text")
        );

        // Custom: empty -> placeholder -> value
        let custom = PromptPart::Custom {
            id: "c1".into(),
            default_text: None,
            placeholder: Some("Describe bug".into()),
            single_line: Some(false),
        };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&custom, &values, &languages, &instances),
            BlockPreviewSummary::muted("Describe bug")
        );
        let custom_no_ph = PromptPart::Custom {
            id: "c2".into(),
            default_text: None,
            placeholder: None,
            single_line: Some(true),
        };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&custom_no_ph, &values, &languages, &instances),
            BlockPreviewSummary::muted("Empty input")
        );
        values.insert("c1".into(), "\nActual input\nLine 2".into());
        assert_eq!(
            viewer_part_preview_summary_from_maps(&custom, &values, &languages, &instances),
            BlockPreviewSummary::text("Actual input")
        );

        // Quote
        let quote = PromptPart::Quote { id: "q1".into() };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&quote, &values, &languages, &instances),
            BlockPreviewSummary::muted("Empty quote")
        );
        values.insert("q1".into(), "Quoted text\nNext line".into());
        assert_eq!(
            viewer_part_preview_summary_from_maps(&quote, &values, &languages, &instances),
            BlockPreviewSummary::text("Quoted text")
        );

        // Code
        let code_no_lang = PromptPart::Code {
            id: "code1".into(),
            language: None,
        };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&code_no_lang, &values, &languages, &instances),
            BlockPreviewSummary::muted("Empty code block")
        );
        let code_with_lang = PromptPart::Code {
            id: "code2".into(),
            language: Some("rust".into()),
        };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&code_with_lang, &values, &languages, &instances),
            BlockPreviewSummary::muted("rust · Empty code block")
        );
        values.insert("code2".into(), "\nfn main() {\n}".into());
        languages.insert("code2".into(), "typescript".into());
        assert_eq!(
            viewer_part_preview_summary_from_maps(&code_with_lang, &values, &languages, &instances),
            BlockPreviewSummary::text("typescript · fn main() {")
        );
        values.insert("code1".into(), "echo hi".into());
        assert_eq!(
            viewer_part_preview_summary_from_maps(&code_no_lang, &values, &languages, &instances),
            BlockPreviewSummary::text("echo hi")
        );

        // Hr
        let hr = PromptPart::Hr { id: "hr1".into() };
        assert_eq!(
            viewer_part_preview_summary_from_maps(&hr, &values, &languages, &instances),
            BlockPreviewSummary::hr()
        );

        // Repeatable
        let rep = PromptPart::Repeatable {
            id: "rep1".into(),
            template_parts: vec![
                PromptPart::Heading {
                    id: "rh1".into(),
                    text: "File".into(),
                    level: 3,
                    exclude_if_next_empty: None,
                },
                PromptPart::Code {
                    id: "rc1".into(),
                    language: Some("diff".into()),
                },
            ],
        };
        instances.insert("rep1".into(), vec!["inst-1".into(), "inst-2".into()]);
        assert_eq!(
            viewer_part_preview_summary_from_maps(&rep, &values, &languages, &instances),
            BlockPreviewSummary::text("2 instances · H3 · File")
        );

        let rep_user_only = PromptPart::Repeatable {
            id: "rep2".into(),
            template_parts: vec![PromptPart::Custom {
                id: "rcust".into(),
                default_text: None,
                placeholder: None,
                single_line: None,
            }],
        };
        instances.insert("rep2".into(), vec!["inst-1".into()]);
        assert_eq!(
            viewer_part_preview_summary_from_maps(&rep_user_only, &values, &languages, &instances),
            BlockPreviewSummary::muted("1 instance · Custom Input")
        );
        values.insert(composite_key("rep2", "inst-1", "rcust"), "User val".into());
        assert_eq!(
            viewer_part_preview_summary_from_maps(&rep_user_only, &values, &languages, &instances),
            BlockPreviewSummary::text("1 instance · User val")
        );
    }
}

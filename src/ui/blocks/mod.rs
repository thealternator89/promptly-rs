pub mod code;
pub mod custom;
pub mod fixed;
pub mod frame;
pub mod heading;
pub mod hr;
pub mod quote;
pub mod repeatable;

pub use frame::BlockFrame;

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

use gpui_kit::*;
use uuid::Uuid;

use crate::model::PromptPart;

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

    pub fn badge(&self) -> &'static str {
        match self {
            EditorPartState::Heading(_) => BlockKind::Heading.badge(),
            EditorPartState::Fixed(_) => BlockKind::Fixed.badge(),
            EditorPartState::Custom(_) => BlockKind::Custom.badge(),
            EditorPartState::Quote { .. } => BlockKind::Quote.badge(),
            EditorPartState::Code(_) => BlockKind::Code.badge(),
            EditorPartState::Hr { .. } => BlockKind::Hr.badge(),
            EditorPartState::Repeatable { .. } => BlockKind::Repeatable.badge(),
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

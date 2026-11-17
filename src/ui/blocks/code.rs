use gpui_kit::component::{
    ActiveTheme as _, IndexPath, Sizable as _, h_flex,
    label::Label,
    searchable_list::SearchableVec,
    select::{Select, SelectState},
};
use gpui_kit::*;

use crate::model::Language;

pub struct CodeEditorState {
    pub id: String,
    pub language_select: Entity<SelectState<SearchableVec<SharedString>>>,
}

impl CodeEditorState {
    pub fn new(id: String, language: Option<&str>, window: &mut Window, cx: &mut App) -> Self {
        let mut items: Vec<SharedString> = Vec::with_capacity(Language::ALL.len() + 1);
        items.push(SharedString::from("None"));
        for lang in Language::ALL {
            items.push(SharedString::from(lang.as_str()));
        }

        let selected_idx = language
            .and_then(|l| {
                Language::ALL
                    .iter()
                    .position(|cand| cand.as_str().eq_ignore_ascii_case(l))
                    .map(|pos| IndexPath::default().row(pos + 1))
            })
            .unwrap_or_else(|| IndexPath::default().row(0));

        let language_select =
            cx.new(|cx| SelectState::new(SearchableVec::new(items), Some(selected_idx), window, cx));

        Self {
            id,
            language_select,
        }
    }

    pub fn selected_language(&self, cx: &App) -> Option<String> {
        let val = self
            .language_select
            .read(cx)
            .selected_value()
            .map(|s| s.to_string())?;
        if val == "None" || val.is_empty() {
            None
        } else {
            Some(val)
        }
    }
}

pub fn render_code_editor(state: &CodeEditorState, cx: &App) -> AnyElement {
    h_flex()
        .gap_3()
        .items_center()
        .child(
            Label::new("Default language:")
                .text_xs()
                .text_color(cx.theme().muted_foreground),
        )
        .child(div().w(px(180.)).child(Select::new(&state.language_select).small()))
        .into_any_element()
}

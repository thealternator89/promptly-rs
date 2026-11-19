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

pub fn render_code_viewer(
    key: &str,
    session_entity: &Entity<crate::model::ViewerSession>,
    session: &crate::model::ViewerSession,
    cx: &App,
) -> AnyElement {
    use gpui_kit::component::{
        button::Button,
        v_flex,
    };

    let Some(textarea) = session.multi_inputs.get(key) else {
        return div().into_any_element();
    };
    let Some(select_state) = session.language_selects.get(key) else {
        return div().into_any_element();
    };

    let has_raw = session.raw_values.contains_key(key);
    let key_owned = key.to_string();
    let paste_key = key_owned.clone();
    let mono_font = cx.theme().mono_font_family.clone();

    v_flex()
        .gap_2()
        .child(super::tab_textarea(textarea, move |t| {
            t.font_family(mono_font).on_paste({
                let session_entity = session_entity.clone();
                let k = paste_key;
                move |clipboard, window, cx| {
                    let Some(text) = clipboard.text() else {
                        return false;
                    };
                    session_entity.update(cx, |sess, cx| {
                        sess.apply_code_paste(&k, &text, window, cx)
                    })
                }
            })
        }))
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().w(px(170.)).child(Select::new(select_state).small()))
                .child(
                    Button::new(SharedString::from(format!("code-indent-{key_owned}")))
                        .small()
                        .outline()
                        .label(if has_raw {
                            "Reset Indentation"
                        } else {
                            "Fix Indentation"
                        })
                        .on_click({
                            let session_entity = session_entity.clone();
                            let k = key_owned;
                            move |_, window, cx| {
                                session_entity.update(cx, |sess, cx| {
                                    if has_raw {
                                        sess.reset_code_indentation(&k, window, cx);
                                    } else {
                                        sess.fix_code_indentation(&k, window, cx);
                                    }
                                });
                            }
                        }),
                ),
        )
        .into_any_element()
}

use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::*;

pub struct FixedEditorState {
    pub id: String,
    pub text_input: Entity<TextareaState>,
}

impl FixedEditorState {
    pub fn new(id: String, text: &str, window: &mut Window, cx: &mut App) -> Self {
        let initial = text.to_string();
        let text_input = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .placeholder("Enter static boilerplate text...");
            state.set_value(initial, window, cx);
            state
        });
        Self { id, text_input }
    }
}

pub fn render_fixed_editor(state: &FixedEditorState) -> AnyElement {
    Textarea::new(&state.text_input)
        .h(px(96.))
        .into_any_element()
}

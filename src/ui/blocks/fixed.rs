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

pub fn render_fixed_viewer(text: &str, cx: &App) -> AnyElement {
    use gpui_kit::component::{ActiveTheme as _, label::Label};

    div()
        .w_full()
        .p_3()
        .rounded_md()
        .bg(cx.theme().background)
        .border_1()
        .border_color(cx.theme().border)
        .child(Label::new(text.to_string()).text_sm())
        .into_any_element()
}

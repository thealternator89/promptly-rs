use gpui_kit::component::{
    Sizable as _,
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    v_flex,
};
use gpui_kit::*;

pub struct CustomEditorState {
    pub id: String,
    pub default_text_input: Entity<InputState>,
    pub placeholder_input: Entity<InputState>,
    pub single_line: bool,
}

impl CustomEditorState {
    pub fn new(
        id: String,
        default_text: Option<&str>,
        placeholder: Option<&str>,
        single_line: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let def_val = default_text.unwrap_or_default().to_string();
        let ph_val = placeholder.unwrap_or_default().to_string();

        let default_text_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Default text (optional)...");
            state.set_value(def_val, window, cx);
            state
        });
        let placeholder_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Placeholder (optional)...");
            state.set_value(ph_val, window, cx);
            state
        });

        Self {
            id,
            default_text_input,
            placeholder_input,
            single_line,
        }
    }
}

pub fn render_custom_editor(
    state: &CustomEditorState,
    on_toggle_single_line: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let id = &state.id;
    let single_line = state.single_line;

    v_flex()
        .gap_2()
        .child(
            h_flex()
                .gap_2()
                .child(div().flex_1().child(Input::new(&state.default_text_input).small()))
                .child(div().flex_1().child(Input::new(&state.placeholder_input).small())),
        )
        .child(
            Checkbox::new(SharedString::from(format!("{id}-single-line")))
                .checked(single_line)
                .label("Single line")
                .on_click(move |checked, window, cx| {
                    on_toggle_single_line(*checked, window, cx);
                }),
        )
        .into_any_element()
}

pub fn render_custom_viewer(
    key: &str,
    single_line: bool,
    session: &crate::model::ViewerSession,
) -> AnyElement {
    if single_line {
        if let Some(input) = session.single_inputs.get(key) {
            return Input::new(input).into_any_element();
        }
    } else if let Some(textarea) = session.multi_inputs.get(key) {
        return super::tab_textarea(textarea, |t| t);
    }
    div().into_any_element()
}

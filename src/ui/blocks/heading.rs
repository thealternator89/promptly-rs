use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    v_flex,
};
use gpui_kit::*;

pub struct HeadingEditorState {
    pub id: String,
    pub level: u8,
    pub exclude_if_next_empty: bool,
    pub text_input: Entity<InputState>,
}

impl HeadingEditorState {
    pub fn new(
        id: String,
        text: &str,
        level: u8,
        exclude_if_next_empty: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let initial = text.to_string();
        let text_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Heading text...");
            state.set_value(initial, window, cx);
            state
        });
        Self {
            id,
            level: level.clamp(1, 3),
            exclude_if_next_empty,
            text_input,
        }
    }
}

pub fn render_heading_editor(
    state: &HeadingEditorState,
    on_set_level: impl Fn(u8, &mut Window, &mut App) + Clone + 'static,
    on_toggle_exclude: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let id = &state.id;
    let level = state.level;
    let exclude = state.exclude_if_next_empty;

    v_flex()
        .gap_2()
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    h_flex()
                        .gap_1()
                        .children([1u8, 2u8, 3u8].into_iter().map(|lvl| {
                            let is_selected = level == lvl;
                            let cb = on_set_level.clone();
                            let btn = Button::new(SharedString::from(format!("{id}-h{lvl}")))
                                .small()
                                .label(format!("H{lvl}"))
                                .on_click(move |_, window, cx| cb(lvl, window, cx));
                            if is_selected {
                                btn.primary()
                            } else {
                                btn.outline()
                            }
                        })),
                )
                .child(div().flex_1().child(Input::new(&state.text_input).small())),
        )
        .child(
            Checkbox::new(SharedString::from(format!("{id}-exclude")))
                .checked(exclude)
                .label("Exclude if next part is empty")
                .on_click(move |checked, window, cx| {
                    on_toggle_exclude(*checked, window, cx);
                }),
        )
        .into_any_element()
}

pub fn render_heading_viewer(text: &str, level: u8) -> AnyElement {
    use gpui_kit::component::StyledExt;
    use gpui_kit::component::label::Label;

    let label = Label::new(text.to_string()).font_bold();
    let styled = match level {
        1 => label.text_xl(),
        2 => label.text_lg(),
        _ => label.text_base(),
    };
    styled.into_any_element()
}

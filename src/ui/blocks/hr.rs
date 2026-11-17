use gpui_kit::component::{ActiveTheme as _, label::Label, v_flex};
use gpui_kit::*;

pub fn render_hr_editor(cx: &App) -> AnyElement {
    let theme = cx.theme();
    v_flex()
        .w_full()
        .gap_2()
        .child(
            div()
                .w_full()
                .h(px(2.))
                .rounded_full()
                .bg(theme.muted_foreground.opacity(0.35)),
        )
        .child(
            Label::new("Horizontal rule divider (---) in compiled Markdown output.")
                .text_xs()
                .text_color(theme.muted_foreground),
        )
        .into_any_element()
}

pub fn render_hr_viewer(cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .w_full()
        .py_2()
        .child(
            div()
                .w_full()
                .h(px(2.))
                .rounded_full()
                .bg(theme.muted_foreground.opacity(0.45)),
        )
        .into_any_element()
}

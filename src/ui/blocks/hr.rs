use gpui_kit::component::{ActiveTheme as _, label::Label};
use gpui_kit::*;

pub fn render_hr_editor(cx: &App) -> AnyElement {
    Label::new("Horizontal rule divider (---) in compiled Markdown output.")
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .into_any_element()
}

pub fn render_hr_viewer(cx: &App) -> AnyElement {
    use gpui_kit::component::ActiveTheme as _;

    div()
        .w_full()
        .py_2()
        .child(div().w_full().h(px(1.)).bg(cx.theme().border))
        .into_any_element()
}

use gpui_kit::component::{ActiveTheme as _, label::Label};
use gpui_kit::*;

pub fn render_hr_editor(cx: &App) -> AnyElement {
    Label::new("Horizontal rule divider (---) in compiled Markdown output.")
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .into_any_element()
}

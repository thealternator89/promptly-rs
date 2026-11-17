use gpui_kit::component::{ActiveTheme as _, label::Label};
use gpui_kit::*;

pub fn render_quote_editor(cx: &App) -> AnyElement {
    Label::new("User-fillable multi-line blockquote in Viewer (lines prefixed with '> ').")
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .into_any_element()
}

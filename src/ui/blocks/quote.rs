use gpui_kit::component::{ActiveTheme as _, label::Label};
use gpui_kit::*;

pub fn render_quote_editor(cx: &App) -> AnyElement {
    Label::new("User-fillable multi-line blockquote in Viewer (lines prefixed with '> ').")
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .into_any_element()
}

pub fn render_quote_viewer(key: &str, session: &crate::model::ViewerSession, cx: &App) -> AnyElement {
    let Some(textarea) = session.multi_inputs.get(key) else {
        return div().into_any_element();
    };

    div()
        .w_full()
        .pl_3()
        .border_l_4()
        .border_color(cx.theme().primary)
        .italic()
        .child(super::tab_textarea(textarea, |t| t))
        .into_any_element()
}

pub mod export_modal;
pub mod import_modal;
pub mod save_modal;
pub mod settings_modal;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

/// Renders a centered modal backdrop and card container.
pub fn modal_backdrop(
    id: &'static str,
    on_close: impl Fn(&mut Window, &mut App) + 'static,
    content: impl IntoElement,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    div()
        .id(id)
        .absolute()
        .inset_0()
        .bg(gpui_kit::black().opacity(0.55))
        .flex()
        .items_center()
        .justify_center()
        .p_6()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            on_close(window, cx);
        })
        .child(
            div()
                .w(px(580.))
                .max_w_full()
                .rounded_xl()
                .bg(theme.popover)
                .border_1()
                .border_color(theme.border)
                .shadow_xl()
                .p_5()
                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .child(content),
        )
        .into_any_element()
}

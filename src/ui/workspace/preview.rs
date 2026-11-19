use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt,
    button::Button,
    h_flex,
    input::Textarea,
    label::Label,
    v_flex,
};
use gpui_kit::*;

use crate::app::AppShell;

/// Renders the right-hand Live Preview pane with compiled and sanitized Markdown output.
pub fn render_live_preview_pane(
    shell: &mut AppShell,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let domains = shell.settings.disallowed_domain_list();
    let compiled = shell
        .active_session()
        .map(|se| se.read(cx).compile_sanitized(&domains))
        .unwrap_or_default();

    shell.sync_preview_textarea(&compiled, window, cx);

    let theme = cx.theme();
    let border_color = theme.border;
    let muted_fg = theme.muted_foreground;
    let secondary_bg = theme.secondary;
    let mono_font = theme.mono_font_family.clone();
    let copied = shell.copied_to_clipboard;

    v_flex()
        .size_full()
        .bg(theme.background)
        .border_l_1()
        .border_color(border_color)
        .child(
            h_flex()
                .px_4()
                .py_2p5()
                .border_b_1()
                .border_color(border_color)
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    v_flex()
                        .child(Label::new("Live Preview").text_sm().font_semibold())
                        .child(
                            Label::new("Compiled & Sanitized Markdown")
                                .text_xs()
                                .text_color(muted_fg),
                        ),
                )
                .child(
                    Button::new("preview-copy")
                        .xsmall()
                        .outline()
                        .icon(Icon::new(if copied {
                            IconName::Check
                        } else {
                            IconName::Copy
                        }))
                        .label(if copied { "Copied!" } else { "Copy" })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.copy_compiled_prompt(cx);
                        })),
                ),
        )
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .p_4()
                .child(
                    Textarea::new(&shell.preview_textarea)
                        .readonly(true)
                        .flex_1()
                        .min_h_0()
                        .bg(secondary_bg)
                        .text_xs()
                        .font_family(mono_font),
                ),
        )
        .into_any_element()
}

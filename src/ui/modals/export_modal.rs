use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Textarea, TextareaState},
    label::Label,
    v_flex,
};
use gpui_kit::*;

use crate::app::AppShell;
use crate::ui::modals::modal_backdrop;

pub struct ExportModal {
    app: WeakEntity<AppShell>,
    json: String,
    json_textarea: Entity<TextareaState>,
    copied: bool,
    _copy_reset_task: Option<Task<()>>,
}

impl ExportModal {
    pub fn new(
        app: WeakEntity<AppShell>,
        json: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial = json.clone();
        let json_textarea = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx);
            state.set_value(initial, window, cx);
            state
        });
        Self {
            app,
            json,
            json_textarea,
            copied: false,
            _copy_reset_task: None,
        }
    }

    fn close(&self, cx: &mut Context<Self>) {
        if let Some(app) = self.app.upgrade() {
            app.update(cx, |shell, cx| shell.close_modal(cx));
        }
    }

    fn copy_json(&mut self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.json.clone()));
        self.copied = true;
        self._copy_reset_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let _ = this.update(cx, |modal, cx| {
                modal.copied = false;
                modal._copy_reset_task = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
}

impl Render for ExportModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_weak = self.app.clone();
        let copied = self.copied;

        let body = v_flex()
            .gap_4()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new("Export Prompt JSON").text_lg().font_semibold())
                    .child(
                        Button::new("export-close-x")
                            .xsmall()
                            .ghost()
                            .icon(Icon::new(IconName::Close))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close(cx);
                            })),
                    ),
            )
            .child(
                Textarea::new(&self.json_textarea)
                    .h(px(280.))
                    .font_family(cx.theme().mono_font_family.clone()),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("export-close")
                            .small()
                            .ghost()
                            .label("Close")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close(cx);
                            })),
                    )
                    .child(
                        Button::new("export-copy")
                            .small()
                            .primary()
                            .icon(Icon::new(if copied {
                                IconName::Check
                            } else {
                                IconName::Copy
                            }))
                            .label(if copied {
                                "Copied!"
                            } else {
                                "Copy to Clipboard"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.copy_json(cx);
                            })),
                    ),
            );

        modal_backdrop(
            "export-modal-backdrop",
            move |_, cx| {
                if let Some(app) = app_weak.upgrade() {
                    app.update(cx, |shell, cx| shell.close_modal(cx));
                }
            },
            body,
            cx,
        )
    }
}

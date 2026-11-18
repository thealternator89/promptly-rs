use std::collections::HashSet;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Textarea, TextareaState},
    label::Label,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::app::AppShell;
use crate::model::import_and_normalize_prompt;
use crate::ui::modals::modal_backdrop;

pub struct ImportModal {
    app: WeakEntity<AppShell>,
    json_input: Entity<TextareaState>,
    error_message: Option<String>,
}

impl ImportModal {
    pub fn new(app: WeakEntity<AppShell>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let json_input = cx.new(|cx| {
            TextareaState::new(window, cx).placeholder("Paste Promptly JSON template here...")
        });
        Self {
            app,
            json_input,
            error_message: None,
        }
    }

    fn close(&self, cx: &mut Context<Self>) {
        if let Some(app) = self.app.upgrade() {
            app.update(cx, |shell, cx| shell.close_modal(cx));
        }
    }

    fn handle_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw_json = self.json_input.read(cx).value().to_string();
        let Some(app) = self.app.upgrade() else {
            return;
        };

        let existing_ids: HashSet<String> = app
            .read(cx)
            .prompts
            .iter()
            .map(|p| p.id.clone())
            .collect();

        match import_and_normalize_prompt(&raw_json, &existing_ids) {
            Ok(prompt) => {
                self.error_message = None;
                app.update(cx, |shell, cx| {
                    shell.finish_import_prompt(prompt, window, cx);
                });
            }
            Err(err) => {
                self.error_message = Some(err.to_string());
                cx.notify();
            }
        }
    }
}

impl Render for ImportModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted_fg = theme.muted_foreground;
        let app_weak = self.app.clone();

        let body = v_flex()
            .gap_4()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new("Import Prompt JSON").text_lg().font_semibold())
                    .child(
                        Button::new("import-close-x")
                            .xsmall()
                            .ghost()
                            .icon(Icon::new(IconName::Close))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close(cx);
                            })),
                    ),
            )
            .child(
                Label::new("Paste a Promptly JSON template below. Missing IDs will be generated automatically.")
                    .text_xs()
                    .text_color(muted_fg),
            )
            .when_some(self.error_message.clone(), |el, err| {
                el.child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .bg(theme.danger.opacity(0.12))
                        .border_1()
                        .border_color(theme.danger)
                        .child(Label::new(err).text_xs().text_color(theme.danger)),
                )
            })
            .child(
                Textarea::new(&self.json_input)
                    .h(px(240.))
                    .font_family(theme.mono_font_family.clone()),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("import-cancel")
                            .small()
                            .ghost()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close(cx);
                            })),
                    )
                    .child(
                        Button::new("import-submit")
                            .small()
                            .primary()
                            .icon(Icon::new(IconName::Upload))
                            .label("Import Prompt")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.handle_import(window, cx);
                            })),
                    ),
            );

        modal_backdrop(
            "import-modal-backdrop",
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

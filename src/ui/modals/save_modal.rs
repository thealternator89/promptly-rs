use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState, Textarea, TextareaState},
    label::Label,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::app::AppShell;
use crate::ui::modals::modal_backdrop;

pub struct SaveToLibraryModal {
    app: WeakEntity<AppShell>,
    title_input: Entity<InputState>,
    description_input: Entity<TextareaState>,
    error_message: Option<String>,
}

impl SaveToLibraryModal {
    pub fn new(
        app: WeakEntity<AppShell>,
        default_title: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Prompt title (required)...");
            if !default_title.is_empty() {
                state.set_value(default_title, window, cx);
            }
            state
        });
        let description_input = cx.new(|cx| {
            TextareaState::new(window, cx).placeholder("Short description (optional)...")
        });

        Self {
            app,
            title_input,
            description_input,
            error_message: None,
        }
    }

    fn close(&self, cx: &mut Context<Self>) {
        if let Some(app) = self.app.upgrade() {
            app.update(cx, |shell, cx| shell.close_modal(cx));
        }
    }

    fn handle_save(&mut self, cx: &mut Context<Self>) {
        let title = self.title_input.read(cx).value().trim().to_string();
        if title.is_empty() {
            self.error_message = Some("Prompt title is required.".to_string());
            cx.notify();
            return;
        }
        let description = self.description_input.read(cx).value().to_string();
        self.error_message = None;

        if let Some(app) = self.app.upgrade() {
            app.update(cx, |shell, cx| {
                shell.promote_active_session_to_library(title, description, cx);
            });
        }
    }
}

impl Render for SaveToLibraryModal {
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
                    .child(Label::new("Add to Library").text_lg().font_semibold())
                    .child(
                        Button::new("save-modal-close-x")
                            .xsmall()
                            .ghost()
                            .icon(Icon::new(IconName::Close))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close(cx);
                            })),
                    ),
            )
            .child(
                Label::new("Save this session as a reusable template in your Library. Your current inputs will be preserved.")
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
                v_flex()
                    .gap_1()
                    .child(Label::new("Title").text_xs().text_color(muted_fg))
                    .child(Input::new(&self.title_input)),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(Label::new("Description").text_xs().text_color(muted_fg))
                    .child(Textarea::new(&self.description_input).h(px(80.))),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("save-modal-cancel")
                            .small()
                            .ghost()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close(cx);
                            })),
                    )
                    .child(
                        Button::new("save-modal-submit")
                            .small()
                            .primary()
                            .icon(Icon::new(IconName::Save))
                            .label("Save to Library")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.handle_save(cx);
                            })),
                    ),
            );

        modal_backdrop(
            "save-modal-backdrop",
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

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Textarea, TextareaState},
    label::Label,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::app::AppShell;
use crate::model::AppSettings;
use crate::ui::modals::modal_backdrop;

pub struct SettingsModal {
    app: WeakEntity<AppShell>,
    domains_input: Entity<TextareaState>,
    legacy_path: Option<String>,
    migration_status: Option<String>,
}

impl SettingsModal {
    pub fn new(
        app: WeakEntity<AppShell>,
        current_settings: &AppSettings,
        legacy_path: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial = current_settings.disallowed_domains.clone();
        let domains_input = cx.new(|cx| {
            let mut state =
                TextareaState::new(window, cx).placeholder("example.com\ninternal.corp");
            if !initial.is_empty() {
                state.set_value(initial, window, cx);
            }
            state
        });

        Self {
            app,
            domains_input,
            legacy_path,
            migration_status: None,
        }
    }

    fn close(&self, cx: &mut Context<Self>) {
        if let Some(app) = self.app.upgrade() {
            app.update(cx, |shell, cx| shell.close_modal(cx));
        }
    }

    fn handle_save(&mut self, cx: &mut Context<Self>) {
        let disallowed_domains = self.domains_input.read(cx).value().to_string();
        let settings = AppSettings { disallowed_domains };
        if let Some(app) = self.app.upgrade() {
            app.update(cx, |shell, cx| {
                shell.save_settings_and_close(settings, cx);
            });
        }
    }

    fn handle_migrate(&mut self, cx: &mut Context<Self>) {
        let Some(app) = self.app.upgrade() else {
            return;
        };
        let status = app.update(cx, |shell, cx| shell.rerun_legacy_migration(cx));
        self.migration_status = Some(status);
        cx.notify();
    }
}

impl Render for SettingsModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let border_color = theme.border;
        let muted_fg = theme.muted_foreground;
        let app_weak = self.app.clone();
        let has_legacy = self.legacy_path.is_some();

        let body = v_flex()
            .gap_4()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new("Settings").text_lg().font_semibold())
                    .child(
                        Button::new("settings-close-x")
                            .xsmall()
                            .ghost()
                            .icon(Icon::new(IconName::Close))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close(cx);
                            })),
                    ),
            )
            // Disallowed Domains
            .child(
                v_flex()
                    .gap_1p5()
                    .child(Label::new("Disallowed Domains").text_sm().font_medium())
                    .child(
                        Label::new(
                            "One domain per line. Links matching these domains or their subdomains are stripped when copying.",
                        )
                        .text_xs()
                        .text_color(muted_fg),
                    )
                    .child(
                        Textarea::new(&self.domains_input)
                            .h(px(140.))
                            .font_family("Menlo"),
                    ),
            )
            // Legacy Data Migration
            .child(
                v_flex()
                    .p_3()
                    .rounded_lg()
                    .bg(theme.secondary)
                    .border_1()
                    .border_color(border_color)
                    .gap_2()
                    .child(
                        Label::new("Legacy Electron Data Migration")
                            .text_sm()
                            .font_medium(),
                    )
                    .child(
                        Label::new(match &self.legacy_path {
                            Some(path) => format!("Detected legacy directory: {path}"),
                            None => "No legacy Electron Promptly directory detected.".to_string(),
                        })
                        .text_xs()
                        .text_color(muted_fg),
                    )
                    .when_some(self.migration_status.clone(), |el, status| {
                        el.child(Label::new(status).text_xs().text_color(theme.foreground))
                    })
                    .child(
                        h_flex().child(
                            Button::new("settings-migrate")
                                .small()
                                .outline()
                                .disabled(!has_legacy)
                                .icon(Icon::new(IconName::FolderSync))
                                .label("Re-run Legacy Migration")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.handle_migrate(cx);
                                })),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("settings-cancel")
                            .small()
                            .ghost()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close(cx);
                            })),
                    )
                    .child(
                        Button::new("settings-save")
                            .small()
                            .primary()
                            .icon(Icon::new(IconName::Save))
                            .label("Save Settings")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.handle_save(cx);
                            })),
                    ),
            );

        modal_backdrop(
            "settings-modal-backdrop",
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

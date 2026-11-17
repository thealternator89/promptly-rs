use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    scroll::ScrollableElement as _,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::app::{AppShell, WorkspaceTarget};

/// Drag payload for reordering saved prompts in the sidebar.
#[derive(Clone)]
pub struct DragPrompt {
    pub id: String,
    pub title: String,
}

pub struct LibrarySidebar {
    app: WeakEntity<AppShell>,
    search_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl LibrarySidebar {
    pub fn new(app: WeakEntity<AppShell>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter prompts..."));
        let sub = cx.subscribe(&search_input, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });

        Self {
            app,
            search_input,
            _subscriptions: vec![sub],
        }
    }
}

impl Render for LibrarySidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(app) = self.app.upgrade() else {
            return v_flex().size_full().into_any_element();
        };

        let (prompts, active_target) = {
            let app_ref = app.read(cx);
            (app_ref.prompts.clone(), app_ref.active_target.clone())
        };

        let query = self.search_input.read(cx).value().trim().to_lowercase();
        let filtered_prompts: Vec<_> = prompts
            .iter()
            .filter(|p| {
                if query.is_empty() {
                    return true;
                }
                p.title.to_lowercase().contains(&query)
                    || p.description.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();

        let is_scratchpad = active_target == WorkspaceTarget::Scratchpad;
        let theme = cx.theme();
        let sidebar_bg = theme.sidebar;
        let border_color = theme.border;
        let muted_fg = theme.muted_foreground;
        let accent_bg = theme.accent;
        let accent_fg = theme.accent_foreground;
        let secondary_bg = theme.secondary;

        v_flex()
            .size_full()
            .bg(sidebar_bg)
            .border_r_1()
            .border_color(border_color)
            // Top action bar: Create (+) & Import
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_between()
                            .child(
                                Label::new("Library")
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(theme.foreground),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("sidebar-import")
                                            .small()
                                            .ghost()
                                            .icon(Icon::new(IconName::Upload))
                                            .label("Import")
                                            .on_click({
                                                let app = app.clone();
                                                move |_, _, cx| {
                                                    app.update(cx, |shell, cx| {
                                                        shell.request_import(cx);
                                                    });
                                                }
                                            }),
                                    )
                                    .child(
                                        Button::new("sidebar-create")
                                            .small()
                                            .primary()
                                            .icon(Icon::new(IconName::Plus))
                                            .label("New")
                                            .on_click({
                                                let app = app.clone();
                                                move |_, window, cx| {
                                                    app.update(cx, |shell, cx| {
                                                        shell.start_new_prompt(window, cx);
                                                    });
                                                }
                                            }),
                                    ),
                            ),
                    )
                    .child(Input::new(&self.search_input).small().cleanable(true)),
            )
            // Pinned Scratchpad + Scrollable Prompts List
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p_2()
                    .gap_1()
                    // Pinned Scratchpad
                    .child(
                        div()
                            .id("sidebar-scratchpad")
                            .px_2p5()
                            .py_2()
                            .rounded_md()
                            .cursor_pointer()
                            .when(is_scratchpad, |el| el.bg(accent_bg).text_color(accent_fg))
                            .when(!is_scratchpad, |el| el.hover(|s| s.bg(secondary_bg)))
                            .on_click({
                                let app = app.clone();
                                move |_, _, cx| {
                                    app.update(cx, |shell, cx| {
                                        shell.select_scratchpad(cx);
                                    });
                                }
                            })
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(
                                                Icon::new(IconName::FilePenLine)
                                                    .small()
                                                    .text_color(if is_scratchpad {
                                                        accent_fg
                                                    } else {
                                                        muted_fg
                                                    }),
                                            )
                                            .child(
                                                Label::new("Scratchpad")
                                                    .text_sm()
                                                    .font_medium(),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .px_1p5()
                                            .py_0p5()
                                            .rounded_sm()
                                            .bg(secondary_bg)
                                            .child(
                                                Label::new("Pinned")
                                                    .text_xs()
                                                    .text_color(muted_fg),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .px_2()
                            .pt_2()
                            .pb_1()
                            .child(
                                Label::new(format!("SAVED PROMPTS ({})", filtered_prompts.len()))
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(muted_fg),
                            ),
                    )
                    .when(filtered_prompts.is_empty(), |el| {
                        let msg = if prompts.is_empty() {
                            "No saved prompts yet. Click New or Import to get started."
                        } else {
                            "No prompts match your filter."
                        };
                        el.child(
                            div().px_3().py_4().child(
                                Label::new(msg)
                                    .text_xs()
                                    .text_color(muted_fg),
                            ),
                        )
                    })
                    .children(filtered_prompts.into_iter().map(|prompt| {
                        let prompt_id = prompt.id.clone();
                        let prompt_title = prompt.title.clone();
                        let is_active =
                            active_target == WorkspaceTarget::SavedPrompt(prompt_id.clone());
                        let part_count = prompt.parts.len();
                        let desc = if prompt.description.trim().is_empty() {
                            None
                        } else {
                            Some(prompt.description.trim().to_string())
                        };

                        let drag_payload = DragPrompt {
                            id: prompt_id.clone(),
                            title: prompt_title.clone(),
                        };

                        div()
                            .id(SharedString::from(format!("sidebar-prompt-{prompt_id}")))
                            .px_2p5()
                            .py_2()
                            .rounded_md()
                            .cursor_pointer()
                            .when(is_active, |el| el.bg(accent_bg).text_color(accent_fg))
                            .when(!is_active, |el| el.hover(|s| s.bg(secondary_bg)))
                            .on_click({
                                let app = app.clone();
                                let pid = prompt_id.clone();
                                move |_, window, cx| {
                                    app.update(cx, |shell, cx| {
                                        shell.select_prompt(pid.clone(), window, cx);
                                    });
                                }
                            })
                            .on_drag(drag_payload, |drag, _, _, cx| {
                                let title = drag.title.clone();
                                cx.new(|_| DragPromptPreview { title })
                            })
                            .drag_over::<DragPrompt>(|style, _, _, cx| {
                                style
                                    .border_t_2()
                                    .border_color(cx.theme().primary)
                            })
                            .on_drop({
                                let app = app.clone();
                                let target_id = prompt_id.clone();
                                move |drag: &DragPrompt, _, cx| {
                                    app.update(cx, |shell, cx| {
                                        shell.reorder_prompt(&drag.id, &target_id, cx);
                                    });
                                }
                            })
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                Label::new(prompt_title)
                                                    .text_sm()
                                                    .font_medium()
                                                    .truncate(),
                                            )
                                            .child(
                                                div()
                                                    .px_1p5()
                                                    .py_0p5()
                                                    .rounded_sm()
                                                    .bg(secondary_bg)
                                                    .child(
                                                        Label::new(format!("{part_count}"))
                                                            .text_xs()
                                                            .text_color(muted_fg),
                                                    ),
                                            ),
                                    )
                                    .when_some(desc, |el, d| {
                                        el.child(
                                            Label::new(d)
                                                .text_xs()
                                                .text_color(if is_active {
                                                    accent_fg
                                                } else {
                                                    muted_fg
                                                })
                                                .truncate(),
                                        )
                                    }),
                            )
                    })),
            )
            // Sidebar Footer: Settings & Version
            .child(
                h_flex()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(border_color)
                    .items_center()
                    .justify_between()
                    .child(
                        Button::new("sidebar-settings")
                            .small()
                            .ghost()
                            .icon(Icon::new(IconName::Settings))
                            .label("Settings")
                            .on_click({
                                let app = app.clone();
                                move |_, _, cx| {
                                    app.update(cx, |shell, cx| {
                                        shell.request_settings(cx);
                                    });
                                }
                            }),
                    )
                    .child(
                        Label::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .text_xs()
                            .text_color(muted_fg),
                    ),
            )
            .into_any_element()
    }
}

struct DragPromptPreview {
    title: String,
}

impl Render for DragPromptPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1p5()
            .rounded_md()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_md()
            .child(
                Label::new(self.title.clone())
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground),
            )
    }
}

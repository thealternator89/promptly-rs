use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    resizable::{h_resizable, resizable_panel},
    scroll::ScrollableElement as _,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::app::{AppShell, WorkspaceMode, WorkspaceTarget};
use crate::model::PromptPart;
use crate::model::session::TemporarySectionKind;
use crate::ui::blocks::{
    BlockFrame,
    code::render_code_viewer,
    custom::render_custom_viewer,
    fixed::render_fixed_viewer,
    heading::render_heading_viewer,
    hr::render_hr_viewer,
    quote::render_quote_viewer,
    repeatable::render_repeatable_viewer,
};
use crate::ui::workspace::preview::render_live_preview_pane;

#[derive(Clone)]
pub struct DragViewerPart {
    pub id: String,
    pub label: String,
}

fn part_badge(part: &PromptPart) -> &'static str {
    match part {
        PromptPart::Heading { .. } => "HEADING",
        PromptPart::Fixed { .. } => "FIXED",
        PromptPart::Custom { .. } => "CUSTOM",
        PromptPart::Quote { .. } => "QUOTE",
        PromptPart::Code { .. } => "CODE",
        PromptPart::Hr { .. } => "HR",
        PromptPart::Repeatable { .. } => "REPEATABLE",
    }
}

pub fn render_viewer_pane(shell: &mut AppShell, cx: &mut Context<AppShell>) -> AnyElement {
    let Some(session_entity) = shell.active_session().cloned() else {
        return v_flex().size_full().into_any_element();
    };

    let is_saved_prompt = matches!(shell.active_target, WorkspaceTarget::SavedPrompt(_));
    let show_preview = shell.show_live_preview;
    let copied = shell.copied_to_clipboard;

    let session = session_entity.read(cx);
    let title = session.title.clone();
    let has_temp = session.has_temporary_sections();
    let parts = session.session_parts.clone();

    let theme = cx.theme();
    let border_color = theme.border;
    let muted_fg = theme.muted_foreground;
    let bg_color = theme.background;
    let secondary_bg = theme.secondary;
    let primary_color = theme.primary;

    let body_content = v_flex()
        .size_full()
        .overflow_y_scrollbar()
        .p_5()
        .gap_3()
        .when(parts.is_empty(), |el| {
            el.child(
                div()
                    .p_6()
                    .rounded_lg()
                    .border_1()
                    .border_dashed()
                    .border_color(border_color)
                    .child(
                        Label::new(
                            "This workspace is empty. Add a temporary section below to start assembling a prompt.",
                        )
                        .text_sm()
                        .text_color(muted_fg),
                    ),
            )
        })
        .children(parts.iter().map(|part| {
            let part_id = part.id().to_string();
            let badge = part_badge(part);
            let drag_payload = DragViewerPart {
                id: part_id.clone(),
                label: badge.to_string(),
            };

            let body = match part {
                PromptPart::Heading { text, level, .. } => render_heading_viewer(text, *level),
                PromptPart::Fixed { text, .. } => render_fixed_viewer(text, cx),
                PromptPart::Custom { single_line, .. } => {
                    render_custom_viewer(&part_id, single_line.unwrap_or(false), session)
                }
                PromptPart::Quote { .. } => render_quote_viewer(&part_id, session, cx),
                PromptPart::Code { .. } => {
                    render_code_viewer(&part_id, &session_entity, session)
                }
                PromptPart::Hr { .. } => render_hr_viewer(cx),
                PromptPart::Repeatable { template_parts, .. } => render_repeatable_viewer(
                    &part_id,
                    template_parts,
                    &session_entity,
                    session,
                    cx,
                ),
            };

            let frame = BlockFrame::new(
                SharedString::from(format!("viewer-block-{part_id}")),
                badge,
            )
            .on_remove({
                let se = session_entity.clone();
                let pid = part_id.clone();
                move |_, _, cx| {
                    se.update(cx, |sess, cx| {
                        sess.remove_part(&pid, cx);
                    });
                }
            })
            .child(body);

            div()
                .id(SharedString::from(format!("drag-viewer-{part_id}")))
                .on_drag(drag_payload, |drag, _, _, cx| {
                    let label = drag.label.clone();
                    cx.new(|_| DragViewerPreview { label })
                })
                .drag_over::<DragViewerPart>(move |style, _, _, _| {
                    style.border_t_2().border_color(primary_color)
                })
                .on_drop({
                    let se = session_entity.clone();
                    let target_id = part_id.clone();
                    move |drag: &DragViewerPart, _, cx| {
                        se.update(cx, |sess, cx| {
                            sess.reorder_part(&drag.id, &target_id, cx);
                        });
                    }
                })
                .child(frame)
        }))
        // Bottom "Add temporary section" bar
        .child(
            v_flex()
                .mt_2()
                .p_3()
                .rounded_lg()
                .bg(secondary_bg)
                .border_1()
                .border_color(border_color)
                .gap_2()
                .child(
                    Label::new("Add temporary section")
                        .text_xs()
                        .font_semibold()
                        .text_color(muted_fg),
                )
                .child(
                    h_flex()
                        .flex_wrap()
                        .gap_2()
                        .children(TemporarySectionKind::ALL.iter().copied().map(|kind| {
                            let se = session_entity.clone();
                            Button::new(SharedString::from(format!(
                                "add-temp-{}",
                                kind.label()
                            )))
                            .small()
                            .outline()
                            .icon(Icon::new(IconName::Plus))
                            .label(kind.label())
                            .on_click(move |_, window, cx| {
                                se.update(cx, |sess, cx| {
                                    sess.add_temporary_section(kind, window, cx);
                                });
                            })
                        })),
                ),
        );

    let preview_pane = render_live_preview_pane(shell, cx);

    v_flex()
        .size_full()
        .bg(bg_color)
        // Sticky Header (full width above blocks + preview split)
        .child(
            h_flex()
                .px_4()
                .py_2p5()
                .border_b_1()
                .border_color(border_color)
                .items_center()
                .justify_between()
                .gap_3()
                .child(Label::new(title).text_base().font_semibold().truncate())
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Button::new("viewer-reset")
                                .small()
                                .ghost()
                                .icon(Icon::new(IconName::RotateCcw))
                                .label("Reset Session")
                                .on_click({
                                    let se = session_entity.clone();
                                    move |_, window, cx| {
                                        se.update(cx, |sess, cx| {
                                            sess.reset_session(window, cx);
                                        });
                                    }
                                }),
                        )
                        .child(
                            Button::new("viewer-add-to-library")
                                .small()
                                .outline()
                                .disabled(!has_temp)
                                .icon(Icon::new(IconName::BookmarkPlus))
                                .label("Add to Library")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.request_save_to_library(window, cx);
                                })),
                        )
                        .when(is_saved_prompt, |el| {
                            el.child(
                                Button::new("viewer-edit")
                                    .small()
                                    .outline()
                                    .icon(Icon::new(IconName::Pencil))
                                    .label("Edit Template")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.set_workspace_mode(WorkspaceMode::Editor, window, cx);
                                    })),
                            )
                        })
                        .child(
                            Button::new("viewer-toggle-preview")
                                .small()
                                .when(show_preview, |b| b.primary())
                                .when(!show_preview, |b| b.outline())
                                .icon(Icon::new(IconName::PanelRight))
                                .label("Preview")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_live_preview(cx);
                                })),
                        )
                        .child(
                            Button::new("viewer-copy")
                                .small()
                                .primary()
                                .icon(Icon::new(if copied {
                                    IconName::Check
                                } else {
                                    IconName::Copy
                                }))
                                .label(if copied {
                                    "Copied to Clipboard!"
                                } else {
                                    "Copy Completed Prompt"
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.copy_compiled_prompt(cx);
                                })),
                        ),
                ),
        )
        // Body Split: Scrollable Block List + Optional Live Preview under the Header
        .child(
            div().flex_1().min_h_0().child(
                h_resizable("viewer-body-split")
                    .child(resizable_panel().child(body_content))
                    .child(
                        resizable_panel()
                            .visible(show_preview)
                            .size(px(360.))
                            .size_range(px(260.)..px(600.))
                            .child(preview_pane),
                    ),
            ),
        )
        .into_any_element()
}

struct DragViewerPreview {
    label: String,
}

impl Render for DragViewerPreview {
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
                Label::new(self.label.clone())
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground),
            )
    }
}

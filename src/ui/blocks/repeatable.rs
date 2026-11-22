use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt,
    button::Button,
    h_flex,
    label::Label,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::ui::blocks::{
    BlockFrame, BlockKind, BlockPreviewSummary, DragBlockPreview, EditorPartState,
    code::render_code_editor,
    custom::render_custom_editor,
    fixed::render_fixed_editor,
    heading::render_heading_editor,
    hr::render_hr_editor,
    quote::render_quote_editor,
    trailing_drop_zone_style,
};
use crate::ui::workspace::editor::{EditorActiveDrag, PromptEditor};

/// Drag payload for reordering nested child blocks inside a `Repeatable` container.
#[derive(Clone)]
pub struct DragRepeatableChild {
    pub parent_id: String,
    pub child_id: String,
    pub source_index: usize,
    pub badge: &'static str,
    pub summary: BlockPreviewSummary,
}

pub fn render_repeatable_editor(
    parent_id: &str,
    children: &[EditorPartState],
    editor: &Entity<PromptEditor>,
    active_child_drag: Option<&str>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let border_color = theme.border;
    let muted_fg = theme.muted_foreground;
    let parent_id_owned = parent_id.to_string();
    let is_dragging_children = active_child_drag.is_some();
    let children_len = children.len();

    v_flex()
        .gap_3()
        .when(children.is_empty(), |el| {
            el.child(
                div()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_dashed()
                    .border_color(border_color)
                    .child(
                        Label::new("No child parts yet. Add blocks below to define the repeatable template.")
                            .text_xs()
                            .text_color(muted_fg),
                    ),
            )
        })
        .children(children.iter().enumerate().map(|(idx, child)| {
            let child_id = child.id().to_string();
            let badge = child.badge();
            let summary = child.preview_summary(cx);

            let frame = if is_dragging_children {
                BlockFrame::new(
                    SharedString::from(format!("nested-{parent_id_owned}-{child_id}")),
                    badge,
                )
                .nested(true)
                .collapsed_preview(Some(summary))
                .dragging_source(active_child_drag == Some(child_id.as_str()))
            } else {
                let drag_payload = DragRepeatableChild {
                    parent_id: parent_id_owned.clone(),
                    child_id: child_id.clone(),
                    source_index: idx,
                    badge,
                    summary,
                };

                let body = match child {
                    EditorPartState::Heading(s) => render_heading_editor(
                        s,
                        {
                            let editor = editor.clone();
                            let pid = parent_id_owned.clone();
                            let cid = child_id.clone();
                            move |lvl, _, cx| {
                                editor.update(cx, |ed, cx| {
                                    ed.set_nested_heading_level(&pid, &cid, lvl, cx);
                                });
                            }
                        },
                        {
                            let editor = editor.clone();
                            let pid = parent_id_owned.clone();
                            let cid = child_id.clone();
                            move |checked, _, cx| {
                                editor.update(cx, |ed, cx| {
                                    ed.set_nested_heading_exclude(&pid, &cid, checked, cx);
                                });
                            }
                        },
                    ),
                    EditorPartState::Fixed(s) => render_fixed_editor(s),
                    EditorPartState::Custom(s) => render_custom_editor(s, {
                        let editor = editor.clone();
                        let pid = parent_id_owned.clone();
                        let cid = child_id.clone();
                        move |checked, _, cx| {
                            editor.update(cx, |ed, cx| {
                                ed.set_nested_custom_single_line(&pid, &cid, checked, cx);
                            });
                        }
                    }),
                    EditorPartState::Code(s) => render_code_editor(s, cx),
                    EditorPartState::Quote { .. } => render_quote_editor(cx),
                    EditorPartState::Hr { .. } => render_hr_editor(cx),
                    EditorPartState::Repeatable { .. } => div().into_any_element(),
                };

                BlockFrame::new(
                    SharedString::from(format!("nested-{parent_id_owned}-{child_id}")),
                    badge,
                )
                .nested(true)
                .on_drag(drag_payload, {
                    let editor = editor.clone();
                    move |drag, _, _, cx| {
                        let preview = cx.new(|_| DragBlockPreview {
                            badge: drag.badge,
                            summary: drag.summary.clone(),
                            compact: true,
                        });
                        let parent_id = drag.parent_id.clone();
                        let child_id = drag.child_id.clone();
                        editor.update(cx, |ed, cx| {
                            ed.start_drag(
                                EditorActiveDrag::RepeatableChild {
                                    parent_id,
                                    child_id,
                                },
                                &preview,
                                cx,
                            );
                        });
                        preview
                    }
                })
                .on_remove({
                    let editor = editor.clone();
                    let pid = parent_id_owned.clone();
                    let cid = child_id.clone();
                    move |_, _, cx| {
                        editor.update(cx, |ed, cx| {
                            ed.remove_nested_part(&pid, &cid, cx);
                        });
                    }
                })
                .child(body)
            };

            let hover_pid = parent_id_owned.clone();
            let hover_cid = child_id.clone();
            div()
                .id(SharedString::from(format!(
                    "drag-nested-{parent_id_owned}-{child_id}"
                )))
                .when(is_dragging_children, |el| el.pt_0p5().pb_0p5())
                .drag_over::<DragRepeatableChild>(move |style, drag, _, cx| {
                    if drag.parent_id != hover_pid || drag.child_id == hover_cid {
                        style
                    } else if drag.source_index < idx {
                        style
                            .pb_0()
                            .border_b_2()
                            .border_color(cx.theme().primary)
                    } else {
                        style
                            .pt_0()
                            .border_t_2()
                            .border_color(cx.theme().primary)
                    }
                })
                .on_drop({
                    let editor = editor.clone();
                    let pid = parent_id_owned.clone();
                    let target_cid = child_id.clone();
                    move |drag: &DragRepeatableChild, _, cx| {
                        if drag.parent_id == pid {
                            editor.update(cx, |ed, cx| {
                                ed.end_drag(cx);
                                ed.reorder_nested_part(&pid, &drag.child_id, &target_cid, cx);
                            });
                        }
                    }
                })
                .child(frame)
        }))
        .when(is_dragging_children && children_len > 1, |el| {
            let hover_pid = parent_id_owned.clone();
            let drop_pid = parent_id_owned.clone();
            let editor = editor.clone();
            el.child(
                trailing_drop_zone_style(
                    SharedString::from(format!("drag-nested-trailing-{parent_id_owned}")),
                    "Drop here to move to end",
                    cx,
                )
                .drag_over::<DragRepeatableChild>(move |style, drag, _, cx| {
                    if drag.parent_id != hover_pid || drag.source_index + 1 >= children_len {
                        style
                    } else {
                        style
                            .border_color(cx.theme().primary)
                            .bg(cx.theme().primary.opacity(0.08))
                    }
                })
                .on_drop(move |drag: &DragRepeatableChild, _, cx| {
                    if drag.parent_id == drop_pid {
                        editor.update(cx, |ed, cx| {
                            ed.end_drag(cx);
                            ed.move_nested_part_to_end(&drop_pid, &drag.child_id, cx);
                        });
                    }
                }),
            )
        })
        // Inline toolbar to append child blocks
        .child(
            h_flex()
                .flex_wrap()
                .gap_1p5()
                .pt_1()
                .child(
                    Label::new("Add child:")
                        .text_xs()
                        .text_color(muted_fg),
                )
                .children(BlockKind::CHILD_KINDS.iter().copied().map(|kind| {
                    let editor = editor.clone();
                    let pid = parent_id_owned.clone();
                    Button::new(SharedString::from(format!(
                        "add-child-{pid}-{}",
                        kind.badge()
                    )))
                    .xsmall()
                    .outline()
                    .icon(Icon::new(IconName::Plus))
                    .label(kind.label())
                    .on_click(move |_, window, cx| {
                        editor.update(cx, |ed, cx| {
                            ed.add_nested_part(&pid, kind, window, cx);
                        });
                    })
                })),
        )
        .into_any_element()
}

pub fn render_repeatable_viewer(
    parent_id: &str,
    template_parts: &[crate::model::PromptPart],
    session_entity: &Entity<crate::model::ViewerSession>,
    session: &crate::model::ViewerSession,
    cx: &App,
) -> AnyElement {
    use crate::model::PromptPart;
    use crate::services::compiler::composite_key;
    use crate::ui::blocks::{
        code::render_code_viewer, custom::render_custom_viewer, fixed::render_fixed_viewer,
        heading::render_heading_viewer, hr::render_hr_viewer, quote::render_quote_viewer,
    };
    use gpui_kit::component::Disableable as _;
    use gpui_kit::component::button::ButtonVariants as _;

    let instances = session
        .repeatable_instances
        .get(parent_id)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let can_remove = instances.len() > 1;
    let theme = cx.theme();
    let border_color = theme.border;
    let muted_fg = theme.muted_foreground;
    let bg = theme.background;
    let parent_id_owned = parent_id.to_string();

    v_flex()
        .gap_3()
        .children(instances.iter().enumerate().map(|(idx, inst_id)| {
            let inst_id_owned = inst_id.clone();
            let pid = parent_id_owned.clone();

            v_flex()
                .id(SharedString::from(format!(
                    "rep-inst-{pid}-{inst_id_owned}"
                )))
                .p_3()
                .rounded_md()
                .bg(bg)
                .border_1()
                .border_color(border_color)
                .gap_2p5()
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            Label::new(format!("Instance #{}", idx + 1))
                                .text_xs()
                                .font_semibold()
                                .text_color(muted_fg),
                        )
                        .child(
                            Button::new(SharedString::from(format!(
                                "remove-inst-{pid}-{inst_id_owned}"
                            )))
                            .xsmall()
                            .ghost()
                            .disabled(!can_remove)
                            .icon(Icon::new(IconName::Trash))
                            .on_click({
                                let se = session_entity.clone();
                                let pid = pid.clone();
                                let iid = inst_id_owned.clone();
                                move |_, _, cx| {
                                    se.update(cx, |sess, cx| {
                                        sess.remove_repeatable_instance(&pid, &iid, cx);
                                    });
                                }
                            }),
                        ),
                )
                .children(template_parts.iter().map(|child| {
                    let key = composite_key(&pid, &inst_id_owned, child.id());
                    let inner = match child {
                        PromptPart::Heading { text, level, .. } => {
                            render_heading_viewer(text, *level)
                        }
                        PromptPart::Fixed { text, .. } => render_fixed_viewer(text, cx),
                        PromptPart::Custom { single_line, .. } => render_custom_viewer(
                            &key,
                            single_line.unwrap_or(false),
                            session,
                        ),
                        PromptPart::Quote { .. } => render_quote_viewer(&key, session, cx),
                        PromptPart::Code { .. } => {
                            render_code_viewer(&key, session_entity, session, cx)
                        }
                        PromptPart::Hr { .. } => render_hr_viewer(cx),
                        PromptPart::Repeatable { .. } => div().into_any_element(),
                    };
                    div()
                        .id(SharedString::from(format!("rep-child-{key}")))
                        .child(inner)
                }))
        }))
        .child(
            Button::new(SharedString::from(format!("add-inst-{parent_id_owned}")))
                .small()
                .outline()
                .icon(Icon::new(IconName::Plus))
                .label("Add another instance")
                .on_click({
                    let se = session_entity.clone();
                    let pid = parent_id_owned;
                    move |_, window, cx| {
                        se.update(cx, |sess, cx| {
                            sess.add_repeatable_instance(&pid, window, cx);
                        });
                    }
                }),
        )
        .into_any_element()
}

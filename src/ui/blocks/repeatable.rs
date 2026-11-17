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
    BlockFrame, BlockKind, EditorPartState,
    code::render_code_editor,
    custom::render_custom_editor,
    fixed::render_fixed_editor,
    heading::render_heading_editor,
    hr::render_hr_editor,
    quote::render_quote_editor,
};
use crate::ui::workspace::editor::PromptEditor;

/// Drag payload for reordering nested child blocks inside a `Repeatable` container.
#[derive(Clone)]
pub struct DragRepeatableChild {
    pub parent_id: String,
    pub child_id: String,
    pub label: String,
}

pub fn render_repeatable_editor(
    parent_id: &str,
    children: &[EditorPartState],
    editor: &Entity<PromptEditor>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let border_color = theme.border;
    let muted_fg = theme.muted_foreground;
    let parent_id_owned = parent_id.to_string();

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
        .children(children.iter().map(|child| {
            let child_id = child.id().to_string();
            let badge = child.badge();
            let drag_payload = DragRepeatableChild {
                parent_id: parent_id_owned.clone(),
                child_id: child_id.clone(),
                label: badge.to_string(),
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

            let frame = BlockFrame::new(
                SharedString::from(format!("nested-{parent_id_owned}-{child_id}")),
                badge,
            )
            .nested(true)
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
            .child(body);

            div()
                .id(SharedString::from(format!("drag-nested-{parent_id_owned}-{child_id}")))
                .on_drag(drag_payload, |drag, _, _, cx| {
                    let label = drag.label.clone();
                    cx.new(|_| DragChildPreview { label })
                })
                .drag_over::<DragRepeatableChild>(|style, _, _, cx| {
                    style.border_t_2().border_color(cx.theme().primary)
                })
                .on_drop({
                    let editor = editor.clone();
                    let pid = parent_id_owned.clone();
                    let target_cid = child_id.clone();
                    move |drag: &DragRepeatableChild, _, cx| {
                        if drag.parent_id == pid {
                            editor.update(cx, |ed, cx| {
                                ed.reorder_nested_part(&pid, &drag.child_id, &target_cid, cx);
                            });
                        }
                    }
                })
                .child(frame)
        }))
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

struct DragChildPreview {
    label: String,
}

impl Render for DragChildPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2p5()
            .py_1()
            .rounded_md()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_md()
            .child(
                Label::new(self.label.clone())
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().foreground),
            )
    }
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
                    match child {
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
                            render_code_viewer(&key, session_entity, session)
                        }
                        PromptPart::Hr { .. } => render_hr_viewer(cx),
                        PromptPart::Repeatable { .. } => div().into_any_element(),
                    }
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

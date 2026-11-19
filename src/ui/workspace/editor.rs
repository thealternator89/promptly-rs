use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState, TextareaState},
    label::Label,
    scroll::ScrollableElement as _,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use uuid::Uuid;

use crate::app::AppShell;
use crate::model::Prompt;
use crate::ui::blocks::{
    BlockFrame, BlockKind, EditorPartState,
    code::render_code_editor,
    custom::render_custom_editor,
    fixed::render_fixed_editor,
    heading::render_heading_editor,
    hr::render_hr_editor,
    quote::render_quote_editor,
    repeatable::render_repeatable_editor,
};

#[derive(Clone)]
pub struct DragTopLevelPart {
    pub id: String,
    pub label: String,
}

pub struct PromptEditor {
    app: WeakEntity<AppShell>,
    editing_prompt_id: Option<String>,
    created_at: Option<u64>,
    title_input: Entity<InputState>,
    description_input: Entity<TextareaState>,
    parts: Vec<EditorPartState>,
    confirm_delete: bool,
    _delete_reset_task: Option<Task<()>>,
    validation_error: Option<String>,
}

impl PromptEditor {
    pub fn new_blank(app: WeakEntity<AppShell>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Prompt title (required)..."));
        let description_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(2, usize::MAX)
                .placeholder("Short description (optional)...")
        });

        Self {
            app,
            editing_prompt_id: None,
            created_at: None,
            title_input,
            description_input,
            parts: Vec::new(),
            confirm_delete: false,
            _delete_reset_task: None,
            validation_error: None,
        }
    }

    pub fn from_prompt(
        app: WeakEntity<AppShell>,
        prompt: &Prompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title = prompt.title.clone();
        let desc = prompt.description.clone();

        let title_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Prompt title (required)...");
            state.set_value(title, window, cx);
            state
        });
        let description_input = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .auto_grow(2, usize::MAX)
                .placeholder("Short description (optional)...");
            state.set_value(desc, window, cx);
            state
        });

        let parts = prompt
            .parts
            .iter()
            .map(|p| EditorPartState::from_prompt_part(p, window, cx))
            .collect();

        Self {
            app,
            editing_prompt_id: Some(prompt.id.clone()),
            created_at: Some(prompt.created_at),
            title_input,
            description_input,
            parts,
            confirm_delete: false,
            _delete_reset_task: None,
            validation_error: None,
        }
    }

    pub fn add_part(&mut self, kind: BlockKind, window: &mut Window, cx: &mut Context<Self>) {
        self.parts.push(EditorPartState::new_blank(kind, window, cx));
        cx.notify();
    }

    pub fn remove_part(&mut self, id: &str, cx: &mut Context<Self>) {
        self.parts.retain(|p| p.id() != id);
        cx.notify();
    }

    pub fn reorder_part(&mut self, drag_id: &str, target_id: &str, cx: &mut Context<Self>) {
        if drag_id == target_id {
            return;
        }
        let Some(from) = self.parts.iter().position(|p| p.id() == drag_id) else {
            return;
        };
        let Some(to) = self.parts.iter().position(|p| p.id() == target_id) else {
            return;
        };
        let item = self.parts.remove(from);
        self.parts.insert(to, item);
        cx.notify();
    }

    pub fn set_heading_level(&mut self, id: &str, level: u8, cx: &mut Context<Self>) {
        if let Some(EditorPartState::Heading(s)) = self.parts.iter_mut().find(|p| p.id() == id) {
            s.level = level.clamp(1, 3);
            cx.notify();
        }
    }

    pub fn set_heading_exclude(&mut self, id: &str, exclude: bool, cx: &mut Context<Self>) {
        if let Some(EditorPartState::Heading(s)) = self.parts.iter_mut().find(|p| p.id() == id) {
            s.exclude_if_next_empty = exclude;
            cx.notify();
        }
    }

    pub fn set_custom_single_line(&mut self, id: &str, single_line: bool, cx: &mut Context<Self>) {
        if let Some(EditorPartState::Custom(s)) = self.parts.iter_mut().find(|p| p.id() == id) {
            s.single_line = single_line;
            cx.notify();
        }
    }

    pub fn add_nested_part(
        &mut self,
        parent_id: &str,
        kind: BlockKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if kind == BlockKind::Repeatable {
            return;
        }
        if let Some(EditorPartState::Repeatable { template_parts, .. }) =
            self.parts.iter_mut().find(|p| p.id() == parent_id)
        {
            template_parts.push(EditorPartState::new_blank(kind, window, cx));
            cx.notify();
        }
    }

    pub fn remove_nested_part(&mut self, parent_id: &str, child_id: &str, cx: &mut Context<Self>) {
        if let Some(EditorPartState::Repeatable { template_parts, .. }) =
            self.parts.iter_mut().find(|p| p.id() == parent_id)
        {
            template_parts.retain(|p| p.id() != child_id);
            cx.notify();
        }
    }

    pub fn reorder_nested_part(
        &mut self,
        parent_id: &str,
        drag_id: &str,
        target_id: &str,
        cx: &mut Context<Self>,
    ) {
        if drag_id == target_id {
            return;
        }
        if let Some(EditorPartState::Repeatable { template_parts, .. }) =
            self.parts.iter_mut().find(|p| p.id() == parent_id)
        {
            let Some(from) = template_parts.iter().position(|p| p.id() == drag_id) else {
                return;
            };
            let Some(to) = template_parts.iter().position(|p| p.id() == target_id) else {
                return;
            };
            let item = template_parts.remove(from);
            template_parts.insert(to, item);
            cx.notify();
        }
    }

    pub fn set_nested_heading_level(
        &mut self,
        parent_id: &str,
        child_id: &str,
        level: u8,
        cx: &mut Context<Self>,
    ) {
        if let Some(EditorPartState::Repeatable { template_parts, .. }) =
            self.parts.iter_mut().find(|p| p.id() == parent_id)
            && let Some(EditorPartState::Heading(s)) =
                template_parts.iter_mut().find(|p| p.id() == child_id)
        {
            s.level = level.clamp(1, 3);
            cx.notify();
        }
    }

    pub fn set_nested_heading_exclude(
        &mut self,
        parent_id: &str,
        child_id: &str,
        exclude: bool,
        cx: &mut Context<Self>,
    ) {
        if let Some(EditorPartState::Repeatable { template_parts, .. }) =
            self.parts.iter_mut().find(|p| p.id() == parent_id)
            && let Some(EditorPartState::Heading(s)) =
                template_parts.iter_mut().find(|p| p.id() == child_id)
        {
            s.exclude_if_next_empty = exclude;
            cx.notify();
        }
    }

    pub fn set_nested_custom_single_line(
        &mut self,
        parent_id: &str,
        child_id: &str,
        single_line: bool,
        cx: &mut Context<Self>,
    ) {
        if let Some(EditorPartState::Repeatable { template_parts, .. }) =
            self.parts.iter_mut().find(|p| p.id() == parent_id)
            && let Some(EditorPartState::Custom(s)) =
                template_parts.iter_mut().find(|p| p.id() == child_id)
        {
            s.single_line = single_line;
            cx.notify();
        }
    }

    fn build_prompt(&self, cx: &App) -> Result<Prompt, String> {
        let title = self.title_input.read(cx).value().trim().to_string();
        if title.is_empty() {
            return Err("Prompt title is required.".to_string());
        }

        let description = self.description_input.read(cx).value().to_string();
        let id = self
            .editing_prompt_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let created_at = self.created_at.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        });

        let parts = self.parts.iter().map(|p| p.to_prompt_part(cx)).collect();
        Ok(Prompt {
            id,
            title,
            description,
            created_at,
            parts,
        })
    }

    fn handle_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.build_prompt(cx) {
            Ok(prompt) => {
                self.validation_error = None;
                if let Some(app) = self.app.upgrade() {
                    app.update(cx, |shell, cx| {
                        shell.save_edited_prompt(prompt, window, cx);
                    });
                }
            }
            Err(err) => {
                self.validation_error = Some(err);
                cx.notify();
            }
        }
    }

    fn handle_cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(app) = self.app.upgrade() {
            let existing_id = self.editing_prompt_id.clone();
            app.update(cx, |shell, cx| {
                shell.cancel_editor(existing_id, cx);
            });
        }
    }

    fn handle_delete_click(&mut self, cx: &mut Context<Self>) {
        let Some(prompt_id) = self.editing_prompt_id.clone() else {
            return;
        };

        if !self.confirm_delete {
            self.confirm_delete = true;
            let task = cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_secs(3)).await;
                let _ = this.update(cx, |ed, cx| {
                    ed.confirm_delete = false;
                    ed._delete_reset_task = None;
                    cx.notify();
                });
            });
            self._delete_reset_task = Some(task);
            cx.notify();
        } else {
            self.confirm_delete = false;
            self._delete_reset_task = None;
            if let Some(app) = self.app.upgrade() {
                app.update(cx, |shell, cx| {
                    shell.delete_prompt(&prompt_id, cx);
                });
            }
        }
    }

    fn handle_export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = Prompt {
            id: self
                .editing_prompt_id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            title: self.title_input.read(cx).value().to_string(),
            description: self.description_input.read(cx).value().to_string(),
            created_at: self.created_at.unwrap_or(0),
            parts: self.parts.iter().map(|p| p.to_prompt_part(cx)).collect(),
        };

        match serde_json::to_string_pretty(&prompt) {
            Ok(json) => {
                if let Some(app) = self.app.upgrade() {
                    app.update(cx, |shell, cx| {
                        shell.open_export_modal(json, window, cx);
                    });
                }
            }
            Err(err) => {
                self.validation_error = Some(format!("Failed to serialize prompt: {err}"));
                cx.notify();
            }
        }
    }
}

impl Render for PromptEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let border_color = theme.border;
        let muted_fg = theme.muted_foreground;
        let is_editing = self.editing_prompt_id.is_some();
        let editor_entity = cx.entity().clone();

        v_flex()
            .size_full()
            .bg(theme.background)
            // Sticky Header
            .child(
                h_flex()
                    .px_4()
                    .py_2p5()
                    .border_b_1()
                    .border_color(border_color)
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        Label::new(if is_editing {
                            "Edit Prompt"
                        } else {
                            "Create New Prompt"
                        })
                        .text_base()
                        .font_semibold(),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Button::new("editor-cancel")
                                    .small()
                                    .ghost()
                                    .label("Cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.handle_cancel(cx);
                                    })),
                            )
                            .when(is_editing, |el| {
                                el.child(
                                    Button::new("editor-export")
                                        .small()
                                        .outline()
                                        .icon(Icon::new(IconName::Download))
                                        .label("Export")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.handle_export(window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("editor-delete")
                                        .small()
                                        .danger()
                                        .icon(Icon::new(IconName::Trash))
                                        .label(if self.confirm_delete {
                                            "Confirm Delete"
                                        } else {
                                            "Delete"
                                        })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.handle_delete_click(cx);
                                        })),
                                )
                            })
                            .child(
                                Button::new("editor-save")
                                    .small()
                                    .primary()
                                    .icon(Icon::new(IconName::Save))
                                    .label("Save Prompt")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.handle_save(window, cx);
                                    })),
                            ),
                    ),
            )
            // Scrollable Body
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p_4()
                    .gap_3()
                    .when_some(self.validation_error.clone(), |el, err| {
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
                    // Metadata Card
                    .child(
                        v_flex()
                            .p_4()
                            .rounded_lg()
                            .bg(theme.secondary)
                            .border_1()
                            .border_color(border_color)
                            .gap_3()
                            .child(
                                Label::new("Template Metadata")
                                    .text_sm()
                                    .font_semibold(),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(Label::new("Title").text_xs().text_color(muted_fg))
                                    .child(Input::new(&self.title_input)),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        Label::new("Description").text_xs().text_color(muted_fg),
                                    )
                                    .child(crate::ui::blocks::tab_textarea(&self.description_input, |t| t)),
                            ),
                    )
                    // Blocks List
                    .child(
                        v_flex()
                            .gap_2()
                            .when(self.parts.is_empty(), |el| {
                                el.child(
                                    div()
                                        .p_4()
                                        .rounded_lg()
                                        .border_1()
                                        .border_dashed()
                                        .border_color(border_color)
                                        .child(
                                            Label::new("No blocks yet. Add a part below to build your prompt template.")
                                                .text_xs()
                                                .text_color(muted_fg),
                                        ),
                                )
                            })
                            .children(self.parts.iter().map(|part| {
                                let part_id = part.id().to_string();
                                let badge = part.badge();
                                let drag_payload = DragTopLevelPart {
                                    id: part_id.clone(),
                                    label: badge.to_string(),
                                };

                                let body = match part {
                                    EditorPartState::Heading(s) => render_heading_editor(
                                        s,
                                        {
                                            let ed = editor_entity.clone();
                                            let pid = part_id.clone();
                                            move |lvl, _, cx| {
                                                ed.update(cx, |this, cx| {
                                                    this.set_heading_level(&pid, lvl, cx);
                                                });
                                            }
                                        },
                                        {
                                            let ed = editor_entity.clone();
                                            let pid = part_id.clone();
                                            move |checked, _, cx| {
                                                ed.update(cx, |this, cx| {
                                                    this.set_heading_exclude(&pid, checked, cx);
                                                });
                                            }
                                        },
                                    ),
                                    EditorPartState::Fixed(s) => render_fixed_editor(s),
                                    EditorPartState::Custom(s) => render_custom_editor(s, {
                                        let ed = editor_entity.clone();
                                        let pid = part_id.clone();
                                        move |checked, _, cx| {
                                            ed.update(cx, |this, cx| {
                                                this.set_custom_single_line(&pid, checked, cx);
                                            });
                                        }
                                    }),
                                    EditorPartState::Code(s) => render_code_editor(s, cx),
                                    EditorPartState::Quote { .. } => render_quote_editor(cx),
                                    EditorPartState::Hr { .. } => render_hr_editor(cx),
                                    EditorPartState::Repeatable { id, template_parts } => {
                                        render_repeatable_editor(
                                            id,
                                            template_parts,
                                            &editor_entity,
                                            cx,
                                        )
                                    }
                                };

                                let frame = BlockFrame::new(
                                    SharedString::from(format!("editor-block-{part_id}")),
                                    badge,
                                )
                                .on_drag(drag_payload, |drag, _, _, cx| {
                                    let label = drag.label.clone();
                                    cx.new(|_| DragTopPreview { label })
                                })
                                .on_remove({
                                    let ed = editor_entity.clone();
                                    let pid = part_id.clone();
                                    move |_, _, cx| {
                                        ed.update(cx, |this, cx| {
                                            this.remove_part(&pid, cx);
                                        });
                                    }
                                })
                                .child(body);

                                div()
                                    .id(SharedString::from(format!("drag-top-{part_id}")))
                                    .drag_over::<DragTopLevelPart>(|style, _, _, cx| {
                                        style.border_t_2().border_color(cx.theme().primary)
                                    })
                                    .on_drop({
                                        let ed = editor_entity.clone();
                                        let target_id = part_id.clone();
                                        move |drag: &DragTopLevelPart, _, cx| {
                                            ed.update(cx, |this, cx| {
                                                this.reorder_part(&drag.id, &target_id, cx);
                                            });
                                        }
                                    })
                                    .child(frame)
                            })),
                    )
                    // Bottom "Add a Part" Toolbar
                    .child(
                        v_flex()
                            .p_3()
                            .rounded_lg()
                            .bg(theme.secondary)
                            .border_1()
                            .border_color(border_color)
                            .gap_2()
                            .child(
                                Label::new("Add a Part")
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(muted_fg),
                            )
                            .child(
                                h_flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .children(BlockKind::ALL.iter().copied().map(|kind| {
                                        Button::new(SharedString::from(format!(
                                            "add-part-{}",
                                            kind.badge()
                                        )))
                                        .small()
                                        .outline()
                                        .icon(Icon::new(IconName::Plus))
                                        .label(kind.label())
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.add_part(kind, window, cx);
                                        }))
                                    })),
                            ),
                    ),
            )
    }
}

struct DragTopPreview {
    label: String,
}

impl Render for DragTopPreview {
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

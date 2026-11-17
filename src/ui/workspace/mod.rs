pub mod editor;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::app::{AppShell, WorkspaceMode, WorkspaceTarget};

/// Renders the center workspace pane (sticky header + body) for Phase 3.
///
/// Full Editor and Viewer block views are implemented in Phases 4 and 5.
pub fn render_workspace_pane(shell: &mut AppShell, cx: &mut Context<AppShell>) -> AnyElement {
    if shell.workspace_mode == WorkspaceMode::Editor
        && let Some(editor) = &shell.editor
    {
        return editor.clone().into_any_element();
    }

    let theme = cx.theme();
    let border_color = theme.border;
    let muted_fg = theme.muted_foreground;

    let (header_title, subtitle, part_count) = match &shell.active_target {
        WorkspaceTarget::Scratchpad => (
            "Scratchpad".to_string(),
            "Temporary workspace for ad-hoc prompt assembly.".to_string(),
            0,
        ),
        WorkspaceTarget::NewPrompt => (
            "Create New Prompt".to_string(),
            "Define a reusable prompt template.".to_string(),
            0,
        ),
        WorkspaceTarget::SavedPrompt(id) => {
            if let Some(prompt) = shell.prompts.iter().find(|p| &p.id == id) {
                (
                    prompt.title.clone(),
                    if prompt.description.trim().is_empty() {
                        "No description provided.".to_string()
                    } else {
                        prompt.description.clone()
                    },
                    prompt.parts.len(),
                )
            } else {
                (
                    "Prompt Not Found".to_string(),
                    "Select a prompt from the sidebar.".to_string(),
                    0,
                )
            }
        }
    };

    let mode = shell.workspace_mode;
    let show_preview = shell.show_live_preview;

    v_flex()
        .size_full()
        .bg(theme.background)
        // Sticky Workspace Header
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
                    v_flex()
                        .gap_0p5()
                        .child(Label::new(header_title.clone()).text_base().font_semibold())
                        .child(Label::new(subtitle).text_xs().text_color(muted_fg)),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Button::new("mode-viewer")
                                .small()
                                .when(mode == WorkspaceMode::Viewer, |b| b.primary())
                                .when(mode != WorkspaceMode::Viewer, |b| b.ghost())
                                .label("Viewer")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.set_workspace_mode(WorkspaceMode::Viewer, window, cx);
                                })),
                        )
                        .child(
                            Button::new("mode-editor")
                                .small()
                                .when(mode == WorkspaceMode::Editor, |b| b.primary())
                                .when(mode != WorkspaceMode::Editor, |b| b.ghost())
                                .label("Editor")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.set_workspace_mode(WorkspaceMode::Editor, window, cx);
                                })),
                        )
                        .child(
                            Button::new("toggle-preview")
                                .small()
                                .when(show_preview, |b| b.primary())
                                .when(!show_preview, |b| b.outline())
                                .icon(Icon::new(IconName::PanelRight))
                                .label("Preview")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_live_preview(cx);
                                })),
                        ),
                ),
        )
        // Workspace Body (Phase 3 placeholder showing active target & mode)
        .child(
            v_flex()
                .flex_1()
                .p_6()
                .gap_3()
                .child(
                    Label::new(format!(
                        "Active Mode: {:?} • Blocks: {}",
                        mode, part_count
                    ))
                    .text_sm()
                    .font_medium(),
                )
                .child(
                    Label::new(format!(
                        "Cached sessions: {}",
                        shell.sessions.len()
                    ))
                    .text_xs()
                    .text_color(muted_fg),
                ),
        )
        .into_any_element()
}

/// Renders the right-hand Live Preview pane when toggled on.
pub fn render_live_preview_pane(shell: &AppShell, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let border_color = theme.border;
    let muted_fg = theme.muted_foreground;

    let active_label = match &shell.active_target {
        WorkspaceTarget::Scratchpad => "Scratchpad".to_string(),
        WorkspaceTarget::NewPrompt => "New Prompt".to_string(),
        WorkspaceTarget::SavedPrompt(id) => shell
            .prompts
            .iter()
            .find(|p| &p.id == id)
            .map(|p| p.title.clone())
            .unwrap_or_else(|| id.clone()),
    };

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
                .child(Label::new("Live Preview").text_sm().font_semibold())
                .child(Label::new("Markdown").text_xs().text_color(muted_fg)),
        )
        .child(
            v_flex()
                .flex_1()
                .p_4()
                .gap_2()
                .child(
                    Label::new(format!("Compiled preview for: {active_label}"))
                        .text_xs()
                        .text_color(muted_fg),
                ),
        )
        .into_any_element()
}

pub mod editor;
pub mod preview;
pub mod viewer;

use gpui_kit::*;

use crate::app::{AppShell, WorkspaceMode};

pub use preview::render_live_preview_pane;

/// Renders the center workspace pane, switching between `PromptEditor` and `PromptViewer`.
pub fn render_workspace_pane(shell: &mut AppShell, cx: &mut Context<AppShell>) -> AnyElement {
    if shell.workspace_mode == WorkspaceMode::Editor
        && let Some(editor) = &shell.editor
    {
        return editor.clone().into_any_element();
    }

    viewer::render_viewer_pane(shell, cx)
}

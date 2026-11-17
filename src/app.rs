use std::collections::HashMap;
use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, StyledExt, TitleBar, h_flex,
    label::Label,
    resizable::{h_resizable, resizable_panel},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use uuid::Uuid;

use crate::model::{AppSettings, Prompt, ViewerSession};
use crate::services::StorageService;
use crate::ui::modals::{
    export_modal::ExportModal,
    import_modal::ImportModal,
    save_modal::SaveToLibraryModal,
    settings_modal::SettingsModal,
};
use crate::ui::sidebar::LibrarySidebar;
use crate::ui::workspace::editor::PromptEditor;
use crate::ui::workspace::{render_live_preview_pane, render_workspace_pane};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceTarget {
    Scratchpad,
    SavedPrompt(String),
    NewPrompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceMode {
    Viewer,
    Editor,
}

pub enum ActiveModal {
    Import(Entity<ImportModal>),
    Export(Entity<ExportModal>),
    SaveToLibrary(Entity<SaveToLibraryModal>),
    Settings(Entity<SettingsModal>),
}

pub struct AppShell {
    pub storage: StorageService,
    pub prompts: Vec<Prompt>,
    pub settings: AppSettings,
    pub active_target: WorkspaceTarget,
    pub workspace_mode: WorkspaceMode,
    pub show_live_preview: bool,
    pub sessions: HashMap<String, Entity<ViewerSession>>,
    pub sidebar: Entity<LibrarySidebar>,
    pub editor: Option<Entity<PromptEditor>>,
    pub active_modal: Option<ActiveModal>,
    pub copied_to_clipboard: bool,
    _copy_reset_task: Option<Task<()>>,
    pub status_message: Option<String>,
}

impl AppShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let storage = StorageService::new().expect("Failed to initialize StorageService");
        if let Err(err) = storage.init_and_migrate() {
            eprintln!("Storage initialization/migration warning: {err}");
        }

        let prompts = storage.load_prompts().unwrap_or_else(|err| {
            eprintln!("Failed to load prompts: {err}");
            Vec::new()
        });
        let settings = storage.load_settings().unwrap_or_default();

        let mut sessions = HashMap::new();
        sessions.insert(
            "scratchpad".to_string(),
            cx.new(|_| ViewerSession::scratchpad()),
        );

        let weak_self = cx.weak_entity();
        let sidebar = cx.new(|cx| LibrarySidebar::new(weak_self, window, cx));

        Self {
            storage,
            prompts,
            settings,
            active_target: WorkspaceTarget::Scratchpad,
            workspace_mode: WorkspaceMode::Viewer,
            show_live_preview: false,
            sessions,
            sidebar,
            editor: None,
            active_modal: None,
            copied_to_clipboard: false,
            _copy_reset_task: None,
            status_message: None,
        }
    }

    pub fn active_session(&self) -> Option<&Entity<ViewerSession>> {
        match &self.active_target {
            WorkspaceTarget::Scratchpad => self.sessions.get("scratchpad"),
            WorkspaceTarget::SavedPrompt(id) => self.sessions.get(id),
            WorkspaceTarget::NewPrompt => None,
        }
    }

    pub fn select_scratchpad(&mut self, cx: &mut Context<Self>) {
        self.active_target = WorkspaceTarget::Scratchpad;
        self.workspace_mode = WorkspaceMode::Viewer;
        self.editor = None;
        self.sessions
            .entry("scratchpad".to_string())
            .or_insert_with(|| cx.new(|_| ViewerSession::scratchpad()));
        self.status_message = None;
        cx.notify();
    }

    pub fn select_prompt(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(prompt) = self.prompts.iter().find(|p| p.id == id).cloned() {
            self.sessions
                .entry(id.clone())
                .or_insert_with(|| cx.new(|cx| ViewerSession::from_prompt(&prompt, window, cx)));
            self.active_target = WorkspaceTarget::SavedPrompt(id);
            self.workspace_mode = WorkspaceMode::Viewer;
            self.editor = None;
            self.status_message = None;
            cx.notify();
        }
    }

    pub fn start_new_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let weak = cx.weak_entity();
        self.editor = Some(cx.new(|cx| PromptEditor::new_blank(weak, window, cx)));
        self.active_target = WorkspaceTarget::NewPrompt;
        self.workspace_mode = WorkspaceMode::Editor;
        self.status_message = None;
        cx.notify();
    }

    pub fn set_workspace_mode(
        &mut self,
        mode: WorkspaceMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if mode == WorkspaceMode::Editor {
            let weak = cx.weak_entity();
            match &self.active_target {
                WorkspaceTarget::SavedPrompt(id) => {
                    if let Some(prompt) = self.prompts.iter().find(|p| &p.id == id).cloned() {
                        self.editor =
                            Some(cx.new(|cx| PromptEditor::from_prompt(weak, &prompt, window, cx)));
                    }
                }
                WorkspaceTarget::NewPrompt | WorkspaceTarget::Scratchpad => {
                    self.active_target = WorkspaceTarget::NewPrompt;
                    self.editor = Some(cx.new(|cx| PromptEditor::new_blank(weak, window, cx)));
                }
            }
        } else {
            self.editor = None;
        }
        self.workspace_mode = mode;
        cx.notify();
    }

    pub fn save_edited_prompt(
        &mut self,
        prompt: Prompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(err) = self.storage.save_prompt(&prompt) {
            self.status_message = Some(format!("Failed to save prompt: {err}"));
            cx.notify();
            return;
        }

        self.prompts = self.storage.load_prompts().unwrap_or_else(|err| {
            self.status_message = Some(format!("Saved, but failed to reload prompts: {err}"));
            self.prompts.clone()
        });

        let id = prompt.id.clone();
        self.sessions
            .insert(id.clone(), cx.new(|cx| ViewerSession::from_prompt(&prompt, window, cx)));
        self.active_target = WorkspaceTarget::SavedPrompt(id);
        self.workspace_mode = WorkspaceMode::Viewer;
        self.editor = None;
        self.status_message = Some(format!("Saved \"{}\"", prompt.title));
        cx.notify();
    }

    pub fn cancel_editor(&mut self, existing_id: Option<String>, cx: &mut Context<Self>) {
        self.editor = None;
        self.workspace_mode = WorkspaceMode::Viewer;
        match existing_id {
            Some(id) => self.active_target = WorkspaceTarget::SavedPrompt(id),
            None => self.active_target = WorkspaceTarget::Scratchpad,
        }
        cx.notify();
    }

    pub fn delete_prompt(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Err(err) = self.storage.delete_prompt(id) {
            self.status_message = Some(format!("Failed to delete prompt: {err}"));
            cx.notify();
            return;
        }
        self.prompts.retain(|p| p.id != id);
        self.sessions.remove(id);
        self.editor = None;
        self.active_target = WorkspaceTarget::Scratchpad;
        self.workspace_mode = WorkspaceMode::Viewer;
        self.status_message = Some("Prompt deleted.".to_string());
        cx.notify();
    }

    pub fn toggle_live_preview(&mut self, cx: &mut Context<Self>) {
        self.show_live_preview = !self.show_live_preview;
        cx.notify();
    }

    pub fn reorder_prompt(&mut self, drag_id: &str, target_id: &str, cx: &mut Context<Self>) {
        if drag_id == target_id {
            return;
        }
        let Some(from_idx) = self.prompts.iter().position(|p| p.id == drag_id) else {
            return;
        };
        let Some(to_idx) = self.prompts.iter().position(|p| p.id == target_id) else {
            return;
        };

        let item = self.prompts.remove(from_idx);
        self.prompts.insert(to_idx, item);

        let ids: Vec<String> = self.prompts.iter().map(|p| p.id.clone()).collect();
        if let Err(err) = self.storage.save_prompts_order(&ids) {
            self.status_message = Some(format!("Failed to save prompt order: {err}"));
        }
        cx.notify();
    }

    pub fn copy_compiled_prompt(&mut self, cx: &mut Context<Self>) {
        let domains = self.settings.disallowed_domain_list();
        let Some(session_entity) = self.active_session() else {
            return;
        };
        let compiled = session_entity.read(cx).compile_sanitized(&domains);
        cx.write_to_clipboard(ClipboardItem::new_string(compiled));

        self.copied_to_clipboard = true;
        self._copy_reset_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let _ = this.update(cx, |shell, cx| {
                shell.copied_to_clipboard = false;
                shell._copy_reset_task = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub fn close_modal(&mut self, cx: &mut Context<Self>) {
        self.active_modal = None;
        cx.notify();
    }

    pub fn request_save_to_library(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let default_title = match &self.active_target {
            WorkspaceTarget::Scratchpad | WorkspaceTarget::NewPrompt => String::new(),
            WorkspaceTarget::SavedPrompt(id) => self
                .prompts
                .iter()
                .find(|p| &p.id == id)
                .map(|p| format!("{} (Copy)", p.title))
                .unwrap_or_default(),
        };
        let weak = cx.weak_entity();
        let modal = cx.new(|cx| SaveToLibraryModal::new(weak, default_title, window, cx));
        self.active_modal = Some(ActiveModal::SaveToLibrary(modal));
        cx.notify();
    }

    pub fn promote_active_session_to_library(
        &mut self,
        title: String,
        description: String,
        cx: &mut Context<Self>,
    ) {
        let source_key = match &self.active_target {
            WorkspaceTarget::Scratchpad => "scratchpad".to_string(),
            WorkspaceTarget::SavedPrompt(id) => id.clone(),
            WorkspaceTarget::NewPrompt => return,
        };

        let Some(session_entity) = self.sessions.get(&source_key).cloned() else {
            return;
        };

        let new_id = Uuid::new_v4().to_string();
        let promoted_parts = session_entity.update(cx, |sess, cx| {
            sess.promote_to_permanent_parts(&new_id, &title, cx)
        });

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let prompt = Prompt {
            id: new_id.clone(),
            title: title.clone(),
            description,
            created_at,
            parts: promoted_parts,
        };

        if let Err(err) = self.storage.save_prompt(&prompt) {
            self.status_message = Some(format!("Failed to save prompt: {err}"));
            cx.notify();
            return;
        }

        // Transfer session to the new prompt ID (and reset Scratchpad if promoted from Scratchpad)
        if source_key == "scratchpad" {
            self.sessions
                .insert("scratchpad".to_string(), cx.new(|_| ViewerSession::scratchpad()));
        }
        self.sessions.insert(new_id.clone(), session_entity);

        self.prompts = self
            .storage
            .load_prompts()
            .unwrap_or_else(|_| self.prompts.clone());
        self.active_target = WorkspaceTarget::SavedPrompt(new_id);
        self.workspace_mode = WorkspaceMode::Viewer;
        self.active_modal = None;
        self.status_message = Some(format!("Added \"{title}\" to Library"));
        cx.notify();
    }

    pub fn request_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let weak = cx.weak_entity();
        let modal = cx.new(|cx| ImportModal::new(weak, window, cx));
        self.active_modal = Some(ActiveModal::Import(modal));
        cx.notify();
    }

    pub fn finish_import_prompt(
        &mut self,
        prompt: Prompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(err) = self.storage.save_prompt(&prompt) {
            self.status_message = Some(format!("Failed to save imported prompt: {err}"));
            cx.notify();
            return;
        }
        self.prompts = self
            .storage
            .load_prompts()
            .unwrap_or_else(|_| self.prompts.clone());
        let id = prompt.id.clone();
        self.sessions
            .insert(id.clone(), cx.new(|cx| ViewerSession::from_prompt(&prompt, window, cx)));
        self.active_target = WorkspaceTarget::SavedPrompt(id);
        self.workspace_mode = WorkspaceMode::Viewer;
        self.editor = None;
        self.active_modal = None;
        self.status_message = Some(format!("Imported \"{}\"", prompt.title));
        cx.notify();
    }

    pub fn open_export_modal(
        &mut self,
        json: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let weak = cx.weak_entity();
        let modal = cx.new(|cx| ExportModal::new(weak, json, window, cx));
        self.active_modal = Some(ActiveModal::Export(modal));
        cx.notify();
    }

    pub fn request_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let weak = cx.weak_entity();
        let settings = self.settings.clone();
        let legacy_path = self
            .storage
            .detected_legacy_dir()
            .map(|p| p.display().to_string());
        let modal = cx.new(|cx| SettingsModal::new(weak, &settings, legacy_path, window, cx));
        self.active_modal = Some(ActiveModal::Settings(modal));
        cx.notify();
    }

    pub fn save_settings_and_close(&mut self, settings: AppSettings, cx: &mut Context<Self>) {
        if let Err(err) = self.storage.save_settings(&settings) {
            self.status_message = Some(format!("Failed to save settings: {err}"));
            cx.notify();
            return;
        }
        self.settings = settings;
        self.active_modal = None;
        self.status_message = Some("Settings saved.".to_string());
        cx.notify();
    }

    pub fn rerun_legacy_migration(&mut self, cx: &mut Context<Self>) -> String {
        let Some(legacy_dir) = self.storage.detected_legacy_dir().map(|p| p.to_path_buf()) else {
            return "No legacy Electron directory found.".to_string();
        };
        match self.storage.migrate_from_directory(&legacy_dir) {
            Ok(report) => {
                self.prompts = self
                    .storage
                    .load_prompts()
                    .unwrap_or_else(|_| self.prompts.clone());
                self.settings = self.storage.load_settings().unwrap_or_default();
                cx.notify();
                format!(
                    "Migrated {} prompt(s), settings: {}, order: {}.",
                    report.prompts_copied, report.settings_copied, report.order_copied
                )
            }
            Err(err) => format!("Migration failed: {err}"),
        }
    }
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let bg = theme.background;
        let border_color = theme.border;
        let muted_fg = theme.muted_foreground;
        let title_bar_bg = theme.title_bar;
        let prompt_count = self.prompts.len();
        let show_preview = self.show_live_preview;
        let status_message = self.status_message.clone();

        let workspace_pane = render_workspace_pane(self, cx);
        let preview_pane = render_live_preview_pane(self, cx);

        let modal_overlay = self.active_modal.as_ref().map(|m| match m {
            ActiveModal::Import(e) => e.clone().into_any_element(),
            ActiveModal::Export(e) => e.clone().into_any_element(),
            ActiveModal::SaveToLibrary(e) => e.clone().into_any_element(),
            ActiveModal::Settings(e) => e.clone().into_any_element(),
        });

        v_flex()
            .relative()
            .size_full()
            .bg(bg)
            // Custom Titlebar
            .child(
                TitleBar::new().child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(Label::new("Promptly").text_sm().font_semibold()),
                ),
            )
            // Multi-pane Split Layout
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("main-split")
                        .child(
                            resizable_panel()
                                .size(px(280.))
                                .size_range(px(220.)..px(420.))
                                .child(self.sidebar.clone()),
                        )
                        .child(resizable_panel().child(workspace_pane))
                        .child(
                            resizable_panel()
                                .visible(show_preview)
                                .size(px(360.))
                                .size_range(px(260.)..px(600.))
                                .child(preview_pane),
                        ),
                ),
            )
            // Global Footer
            .child(
                h_flex()
                    .px_4()
                    .py_1p5()
                    .bg(title_bar_bg)
                    .border_t_1()
                    .border_color(border_color)
                    .items_center()
                    .justify_between()
                    .child(
                        Label::new(format!(
                            "Promptly v{} • {} saved prompt{}",
                            env!("CARGO_PKG_VERSION"),
                            prompt_count,
                            if prompt_count == 1 { "" } else { "s" }
                        ))
                        .text_xs()
                        .text_color(muted_fg),
                    )
                    .when_some(status_message, |el, msg| {
                        el.child(Label::new(msg).text_xs().text_color(muted_fg))
                    }),
            )
            .when_some(modal_overlay, |el, modal| el.child(modal))
    }
}

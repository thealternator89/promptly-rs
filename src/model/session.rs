use std::collections::HashMap;

use gpui_kit::component::{
    IndexPath,
    input::{InputEvent, InputState, TextareaState},
    searchable_list::SearchableVec,
    select::{SelectEvent, SelectState},
};
use gpui_kit::*;
use uuid::Uuid;

use crate::model::{Language, Prompt, PromptPart};
use crate::services::{compile_prompt, fix_indentation, strip_disallowed_urls};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporarySectionKind {
    Custom,
    Quote,
    Code,
    Hr,
}

impl TemporarySectionKind {
    pub const ALL: &'static [TemporarySectionKind] = &[
        TemporarySectionKind::Custom,
        TemporarySectionKind::Quote,
        TemporarySectionKind::Code,
        TemporarySectionKind::Hr,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            TemporarySectionKind::Custom => "Custom Input",
            TemporarySectionKind::Quote => "Quote",
            TemporarySectionKind::Code => "Code Block",
            TemporarySectionKind::Hr => "Rule",
        }
    }
}

/// Cached runtime state for a Prompt Viewer or Scratchpad session.
pub struct ViewerSession {
    pub prompt_id: String,
    pub title: String,
    pub source_parts: Vec<PromptPart>,
    pub session_parts: Vec<PromptPart>,
    pub values: HashMap<String, String>,
    pub languages: HashMap<String, String>,
    pub raw_values: HashMap<String, String>,
    pub repeatable_instances: HashMap<String, Vec<String>>,
    pub single_inputs: HashMap<String, Entity<InputState>>,
    pub multi_inputs: HashMap<String, Entity<TextareaState>>,
    pub language_selects: HashMap<String, Entity<SelectState<SearchableVec<SharedString>>>>,
    _subscriptions: Vec<Subscription>,
}

impl ViewerSession {
    pub fn scratchpad() -> Self {
        Self {
            prompt_id: "scratchpad".to_string(),
            title: "Scratchpad".to_string(),
            source_parts: Vec::new(),
            session_parts: Vec::new(),
            values: HashMap::new(),
            languages: HashMap::new(),
            raw_values: HashMap::new(),
            repeatable_instances: HashMap::new(),
            single_inputs: HashMap::new(),
            multi_inputs: HashMap::new(),
            language_selects: HashMap::new(),
            _subscriptions: Vec::new(),
        }
    }

    pub fn from_prompt(
        prompt: &Prompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut session = Self {
            prompt_id: prompt.id.clone(),
            title: prompt.title.clone(),
            source_parts: prompt.parts.clone(),
            session_parts: prompt.parts.clone(),
            values: HashMap::new(),
            languages: HashMap::new(),
            raw_values: HashMap::new(),
            repeatable_instances: HashMap::new(),
            single_inputs: HashMap::new(),
            multi_inputs: HashMap::new(),
            language_selects: HashMap::new(),
            _subscriptions: Vec::new(),
        };
        session.init_all_controls(window, cx);
        session
    }

    pub fn reset_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.session_parts = self.source_parts.clone();
        self.values.clear();
        self.languages.clear();
        self.raw_values.clear();
        self.repeatable_instances.clear();
        self.single_inputs.clear();
        self.multi_inputs.clear();
        self.language_selects.clear();
        self._subscriptions.clear();
        self.init_all_controls(window, cx);
        cx.notify();
    }

    fn init_all_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parts = self.session_parts.clone();
        for part in &parts {
            self.init_part_controls(part, None, window, cx);
        }
    }

    fn init_part_controls(
        &mut self,
        part: &PromptPart,
        parent_instance: Option<(&str, &str)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = match parent_instance {
            Some((pid, iid)) => format!("{pid}_{iid}_{}", part.id()),
            None => part.id().to_string(),
        };

        match part {
            PromptPart::Custom {
                default_text,
                placeholder,
                single_line,
                ..
            } => {
                let def = default_text.clone().unwrap_or_default();
                let ph = placeholder.clone().unwrap_or_default();
                if !def.is_empty() {
                    self.values.insert(key.clone(), def.clone());
                }

                if single_line.unwrap_or(false) {
                    let input = cx.new(|cx| {
                        let mut state = InputState::new(window, cx).placeholder(ph);
                        if !def.is_empty() {
                            state.set_value(def, window, cx);
                        }
                        state
                    });
                    let k = key.clone();
                    let sub = cx.subscribe(&input, move |this, entity, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            let val = entity.read(cx).value().to_string();
                            this.values.insert(k.clone(), val);
                            cx.notify();
                        }
                    });
                    self.single_inputs.insert(key, input);
                    self._subscriptions.push(sub);
                } else {
                    let textarea = cx.new(|cx| {
                        let mut state = TextareaState::new(window, cx)
                            .auto_grow(3, usize::MAX)
                            .placeholder(ph);
                        if !def.is_empty() {
                            state.set_value(def, window, cx);
                        }
                        state
                    });
                    let k = key.clone();
                    let sub =
                        cx.subscribe(&textarea, move |this, entity, event: &InputEvent, cx| {
                            if matches!(event, InputEvent::Change) {
                                let val = entity.read(cx).value().to_string();
                                this.values.insert(k.clone(), val);
                                cx.notify();
                            }
                        });
                    self.multi_inputs.insert(key, textarea);
                    self._subscriptions.push(sub);
                }
            }
            PromptPart::Quote { .. } => {
                let textarea = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .auto_grow(3, usize::MAX)
                        .placeholder("Enter quote text...")
                });
                let k = key.clone();
                let sub =
                    cx.subscribe(&textarea, move |this, entity, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            let val = entity.read(cx).value().to_string();
                            this.values.insert(k.clone(), val);
                            cx.notify();
                        }
                    });
                self.multi_inputs.insert(key, textarea);
                self._subscriptions.push(sub);
            }
            PromptPart::Code { language, .. } => {
                if let Some(lang) = language {
                    self.languages.insert(key.clone(), lang.clone());
                }

                let textarea = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .auto_grow(5, usize::MAX)
                        .placeholder("Paste or type code...")
                });
                let k = key.clone();
                let sub =
                    cx.subscribe(&textarea, move |this, entity, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            let val = entity.read(cx).value().to_string();
                            this.values.insert(k.clone(), val);
                            cx.notify();
                        }
                    });
                self.multi_inputs.insert(key.clone(), textarea);
                self._subscriptions.push(sub);

                // Language dropdown
                let mut items: Vec<SharedString> = Vec::with_capacity(Language::ALL.len() + 1);
                items.push(SharedString::from("None"));
                for lang in Language::ALL {
                    items.push(SharedString::from(lang.as_str()));
                }
                let selected_idx = language
                    .as_deref()
                    .and_then(|l| {
                        Language::ALL
                            .iter()
                            .position(|cand| cand.as_str().eq_ignore_ascii_case(l))
                            .map(|pos| IndexPath::default().row(pos + 1))
                    })
                    .unwrap_or_else(|| IndexPath::default().row(0));

                let select = cx.new(|cx| {
                    SelectState::new(SearchableVec::new(items), Some(selected_idx), window, cx)
                });
                let k_lang = key.clone();
                let lang_sub = cx.subscribe(
                    &select,
                    move |this, _, event: &SelectEvent<SearchableVec<SharedString>>, cx| {
                        if let SelectEvent::Confirm(Some(selected)) = event {
                            if selected.as_ref() == "None" {
                                this.languages.remove(&k_lang);
                            } else {
                                this.languages.insert(k_lang.clone(), selected.to_string());
                            }
                            cx.notify();
                        }
                    },
                );
                self.language_selects.insert(key, select);
                self._subscriptions.push(lang_sub);
            }
            PromptPart::Repeatable { id, template_parts } => {
                let instance_id = format!("inst-{}", Uuid::new_v4());
                self.repeatable_instances
                    .insert(id.clone(), vec![instance_id.clone()]);
                for child in template_parts {
                    self.init_part_controls(
                        child,
                        Some((id.as_str(), instance_id.as_str())),
                        window,
                        cx,
                    );
                }
            }
            PromptPart::Fixed { .. } | PromptPart::Heading { .. } | PromptPart::Hr { .. } => {}
        }
    }

    pub fn has_temporary_sections(&self) -> bool {
        self.session_parts
            .iter()
            .any(|p| p.id().starts_with("session_"))
    }

    pub fn add_temporary_section(
        &mut self,
        kind: TemporarySectionKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = format!("session_{}", Uuid::new_v4());
        let part = match kind {
            TemporarySectionKind::Custom => PromptPart::Custom {
                id,
                default_text: None,
                placeholder: Some("Enter custom text...".to_string()),
                single_line: Some(false),
            },
            TemporarySectionKind::Quote => PromptPart::Quote { id },
            TemporarySectionKind::Code => PromptPart::Code { id, language: None },
            TemporarySectionKind::Hr => PromptPart::Hr { id },
        };
        self.init_part_controls(&part, None, window, cx);
        self.session_parts.push(part);
        cx.notify();
    }

    pub fn remove_part(&mut self, id: &str, cx: &mut Context<Self>) {
        self.session_parts.retain(|p| p.id() != id);
        cx.notify();
    }

    pub fn reorder_part(&mut self, drag_id: &str, target_id: &str, cx: &mut Context<Self>) {
        if drag_id == target_id {
            return;
        }
        let Some(from) = self.session_parts.iter().position(|p| p.id() == drag_id) else {
            return;
        };
        let Some(to) = self.session_parts.iter().position(|p| p.id() == target_id) else {
            return;
        };
        let item = self.session_parts.remove(from);
        self.session_parts.insert(to, item);
        cx.notify();
    }

    pub fn move_part_to_end(&mut self, drag_id: &str, cx: &mut Context<Self>) {
        if move_part_to_end_in_vec(&mut self.session_parts, drag_id) {
            cx.notify();
        }
    }

    pub fn add_repeatable_instance(
        &mut self,
        parent_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(PromptPart::Repeatable { template_parts, .. }) =
            self.session_parts.iter().find(|p| p.id() == parent_id)
        else {
            return;
        };
        let template_parts = template_parts.clone();
        let instance_id = format!("inst-{}", Uuid::new_v4());
        self.repeatable_instances
            .entry(parent_id.to_string())
            .or_default()
            .push(instance_id.clone());

        for child in &template_parts {
            self.init_part_controls(child, Some((parent_id, instance_id.as_str())), window, cx);
        }
        cx.notify();
    }

    pub fn remove_repeatable_instance(
        &mut self,
        parent_id: &str,
        instance_id: &str,
        cx: &mut Context<Self>,
    ) {
        if let Some(instances) = self.repeatable_instances.get_mut(parent_id)
            && instances.len() > 1
        {
            instances.retain(|id| id != instance_id);
            cx.notify();
        }
    }

    /// Intercepts paste in a `Code` block, running `fix_indentation` and saving raw text
    /// for one-click undo if indentation was modified. Returns `true` if handled.
    pub fn apply_code_paste(
        &mut self,
        key: &str,
        pasted: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let fixed = fix_indentation(pasted);
        if fixed == pasted {
            return false;
        }
        let Some(textarea) = self.multi_inputs.get(key).cloned() else {
            return false;
        };

        // Compute the raw value by simulating inserting `pasted` into the current value,
        // or if the buffer was empty, just `pasted.to_string()`.
        let current = textarea.read(cx).value().to_string();
        let raw_combined = if current.is_empty() {
            pasted.to_string()
        } else {
            format!("{current}{pasted}")
        };
        self.raw_values.insert(key.to_string(), raw_combined);

        textarea.update(cx, |state, cx| {
            state.insert(fixed, window, cx);
        });
        let updated = textarea.read(cx).value().to_string();
        self.values.insert(key.to_string(), updated);
        cx.notify();
        true
    }

    pub fn fix_code_indentation(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(textarea) = self.multi_inputs.get(key).cloned() else {
            return;
        };
        let current = textarea.read(cx).value().to_string();
        let fixed = fix_indentation(&current);
        if fixed != current {
            self.raw_values.insert(key.to_string(), current);
            textarea.update(cx, |state, cx| {
                state.set_value(fixed.clone(), window, cx);
            });
            self.values.insert(key.to_string(), fixed);
            cx.notify();
        }
    }

    pub fn reset_code_indentation(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(raw) = self.raw_values.remove(key) else {
            return;
        };
        if let Some(textarea) = self.multi_inputs.get(key).cloned() {
            textarea.update(cx, |state, cx| {
                state.set_value(raw.clone(), window, cx);
            });
            self.values.insert(key.to_string(), raw);
            cx.notify();
        }
    }

    pub fn compile_sanitized(&self, disallowed_domains: &[String]) -> String {
        let compiled = compile_prompt(
            &self.session_parts,
            &self.values,
            &self.languages,
            &self.repeatable_instances,
        );
        strip_disallowed_urls(&compiled, disallowed_domains)
    }

    /// Converts `session_` temporary IDs to permanent `part_` IDs, stores current `Custom`
    /// values as their new `default_text`, and updates this session in-place so no typed
    /// inputs or repeatable instances are lost when promoting to a saved Prompt.
    pub fn promote_to_permanent_parts(
        &mut self,
        new_prompt_id: &str,
        new_title: &str,
        cx: &mut Context<Self>,
    ) -> Vec<PromptPart> {
        let mut id_map: HashMap<String, String> = HashMap::new();
        for part in &mut self.session_parts {
            let old_id = part.id().to_string();
            if old_id.starts_with("session_") {
                let new_id = format!("part_{}", Uuid::new_v4());
                id_map.insert(old_id, new_id.clone());
                set_part_id(part, new_id);
            }
        }

        for (old_id, new_id) in &id_map {
            if let Some(v) = self.values.remove(old_id) {
                self.values.insert(new_id.clone(), v);
            }
            if let Some(v) = self.languages.remove(old_id) {
                self.languages.insert(new_id.clone(), v);
            }
            if let Some(v) = self.raw_values.remove(old_id) {
                self.raw_values.insert(new_id.clone(), v);
            }
            if let Some(e) = self.single_inputs.remove(old_id) {
                self.single_inputs.insert(new_id.clone(), e);
            }
            if let Some(e) = self.multi_inputs.remove(old_id) {
                self.multi_inputs.insert(new_id.clone(), e);
            }
            if let Some(e) = self.language_selects.remove(old_id) {
                self.language_selects.insert(new_id.clone(), e);
            }
        }

        // Re-bind subscriptions so future edits write to the new permanent keys
        if !id_map.is_empty() {
            self._subscriptions.clear();
            for (k, input) in &self.single_inputs {
                let key = k.clone();
                let sub = cx.subscribe(input, move |this, entity, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        let val = entity.read(cx).value().to_string();
                        this.values.insert(key.clone(), val);
                        cx.notify();
                    }
                });
                self._subscriptions.push(sub);
            }
            for (k, textarea) in &self.multi_inputs {
                let key = k.clone();
                let sub = cx.subscribe(textarea, move |this, entity, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        let val = entity.read(cx).value().to_string();
                        this.values.insert(key.clone(), val);
                        cx.notify();
                    }
                });
                self._subscriptions.push(sub);
            }
            for (k, select) in &self.language_selects {
                let key = k.clone();
                let sub = cx.subscribe(
                    select,
                    move |this, _, event: &SelectEvent<SearchableVec<SharedString>>, cx| {
                        if let SelectEvent::Confirm(Some(selected)) = event {
                            if selected.as_ref() == "None" {
                                this.languages.remove(&key);
                            } else {
                                this.languages.insert(key.clone(), selected.to_string());
                            }
                            cx.notify();
                        }
                    },
                );
                self._subscriptions.push(sub);
            }
        }

        // Persist current Custom field values as their new default_text
        for part in &mut self.session_parts {
            if let PromptPart::Custom {
                id, default_text, ..
            } = part
            {
                *default_text = self
                    .values
                    .get(id)
                    .filter(|s| !s.is_empty())
                    .cloned();
            }
        }

        self.prompt_id = new_prompt_id.to_string();
        self.title = new_title.to_string();
        self.source_parts = self.session_parts.clone();
        cx.notify();
        self.session_parts.clone()
    }
}

fn move_part_to_end_in_vec(parts: &mut Vec<PromptPart>, drag_id: &str) -> bool {
    let Some(from) = parts.iter().position(|p| p.id() == drag_id) else {
        return false;
    };
    if from + 1 >= parts.len() {
        return false;
    }
    let item = parts.remove(from);
    parts.push(item);
    true
}

fn set_part_id(part: &mut PromptPart, new_id: String) {
    match part {
        PromptPart::Fixed { id, .. }
        | PromptPart::Heading { id, .. }
        | PromptPart::Custom { id, .. }
        | PromptPart::Quote { id }
        | PromptPart::Code { id, .. }
        | PromptPart::Hr { id }
        | PromptPart::Repeatable { id, .. } => *id = new_id,
    }
}

#[cfg(test)]
mod tests {
    use super::move_part_to_end_in_vec;
    use crate::model::PromptPart;

    fn hr(id: &str) -> PromptPart {
        PromptPart::Hr { id: id.to_string() }
    }

    #[test]
    fn move_part_to_end_reorders_correctly() {
        let mut parts = vec![hr("a"), hr("b"), hr("c")];

        assert!(move_part_to_end_in_vec(&mut parts, "a"));
        assert_eq!(
            parts.iter().map(PromptPart::id).collect::<Vec<_>>(),
            vec!["b", "c", "a"]
        );

        assert!(move_part_to_end_in_vec(&mut parts, "c"));
        assert_eq!(
            parts.iter().map(PromptPart::id).collect::<Vec<_>>(),
            vec!["b", "a", "c"]
        );

        // Already at end -> no change
        assert!(!move_part_to_end_in_vec(&mut parts, "c"));
        // Missing ID -> no change
        assert!(!move_part_to_end_in_vec(&mut parts, "missing"));
    }
}

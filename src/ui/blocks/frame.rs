use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

type DragConfigurator = Box<dyn FnOnce(Stateful<Div>) -> Stateful<Div>>;
type RemoveHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Single-line summary displayed when a prompt block is collapsed during drag-and-drop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPreviewSummary {
    pub text: String,
    pub is_muted: bool,
    pub is_hr: bool,
}

impl BlockPreviewSummary {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_muted: false,
            is_hr: false,
        }
    }

    pub fn muted(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_muted: true,
            is_hr: false,
        }
    }

    pub fn hr() -> Self {
        Self {
            text: "Horizontal rule".to_string(),
            is_muted: true,
            is_hr: true,
        }
    }
}

/// Extracts the first non-empty trimmed line of `text`, normalizing tabs to spaces.
pub fn first_non_empty_line(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.replace('\t', " "))
        }
    })
}

/// Shared floating drag preview pill rendered beneath the cursor during block drags.
pub struct DragBlockPreview {
    pub badge: &'static str,
    pub summary: BlockPreviewSummary,
    pub compact: bool,
}

impl Render for DragBlockPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted_fg = theme.muted_foreground;

        let badge_el = div()
            .flex_shrink_0()
            .px_1p5()
            .py_0p5()
            .rounded_sm()
            .bg(theme.secondary)
            .child(
                Label::new(self.badge)
                    .text_xs()
                    .font_semibold()
                    .text_color(muted_fg),
            );

        let preview_el = if self.summary.is_hr {
            div()
                .w(px(64.))
                .h(px(2.))
                .rounded_full()
                .bg(muted_fg.opacity(0.4))
                .into_any_element()
        } else {
            let label = Label::new(self.summary.text.clone())
                .truncate()
                .when(self.compact, |l| l.text_xs())
                .when(!self.compact, |l| l.text_sm())
                .when(self.summary.is_muted, |l| {
                    l.text_color(muted_fg.opacity(0.8)).italic()
                })
                .when(!self.summary.is_muted, |l| l.text_color(theme.foreground));
            div().flex_initial().min_w_0().child(label).into_any_element()
        };

        h_flex()
            .max_w(px(360.))
            .items_center()
            .gap_2()
            .when(self.compact, |el| el.px_2p5().py_1())
            .when(!self.compact, |el| el.px_3().py_1p5())
            .rounded_md()
            .bg(theme.popover)
            .border_1()
            .border_color(theme.primary.opacity(0.5))
            .shadow_md()
            .child(badge_el)
            .child(preview_el)
    }
}

/// Renders the compact drop zone appended to the end of a block list while dragging.
pub fn trailing_drop_zone_style(
    id: impl Into<ElementId>,
    label: &'static str,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    div()
        .id(id)
        .w_full()
        .px_3()
        .py_1p5()
        .rounded_md()
        .border_1()
        .border_dashed()
        .border_color(theme.border.opacity(0.6))
        .bg(theme.secondary.opacity(0.25))
        .child(
            Label::new(label)
                .text_xs()
                .text_color(theme.muted_foreground.opacity(0.75)),
        )
}

/// Shared container frame for both top-level and nested blocks.
#[derive(IntoElement)]
pub struct BlockFrame {
    id: ElementId,
    badge: &'static str,
    is_nested: bool,
    show_drag_handle: bool,
    show_remove: bool,
    collapsed_preview: Option<BlockPreviewSummary>,
    is_dragging_source: bool,
    on_drag: Option<DragConfigurator>,
    on_remove: Option<RemoveHandler>,
    children: Vec<AnyElement>,
}

impl BlockFrame {
    pub fn new(id: impl Into<ElementId>, badge: &'static str) -> Self {
        Self {
            id: id.into(),
            badge,
            is_nested: false,
            show_drag_handle: true,
            show_remove: true,
            collapsed_preview: None,
            is_dragging_source: false,
            on_drag: None,
            on_remove: None,
            children: Vec::new(),
        }
    }

    pub fn nested(mut self, is_nested: bool) -> Self {
        self.is_nested = is_nested;
        self
    }

    pub fn show_drag_handle(mut self, show: bool) -> Self {
        self.show_drag_handle = show;
        self
    }

    pub fn show_remove(mut self, show: bool) -> Self {
        self.show_remove = show;
        self
    }

    pub fn collapsed_preview(mut self, summary: Option<BlockPreviewSummary>) -> Self {
        self.collapsed_preview = summary;
        self
    }

    pub fn dragging_source(mut self, is_source: bool) -> Self {
        self.is_dragging_source = is_source;
        self
    }

    pub fn on_drag<T, W>(
        mut self,
        value: T,
        constructor: impl Fn(&T, Point<Pixels>, &mut Window, &mut App) -> Entity<W> + 'static,
    ) -> Self
    where
        T: 'static,
        W: 'static + Render,
    {
        self.on_drag = Some(Box::new(move |el| el.on_drag(value, constructor)));
        self
    }

    pub fn on_remove(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl RenderOnce for BlockFrame {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let bg = if self.is_nested {
            theme.background
        } else {
            theme.secondary.opacity(0.45)
        };
        let border_color = theme.border.opacity(0.55);
        let muted_fg = theme.muted_foreground;

        if let Some(summary) = self.collapsed_preview {
            let badge_bg = if self.is_nested {
                theme.secondary
            } else {
                theme.background.opacity(0.75)
            };
            let badge_id = ElementId::Name(format!("{}-badge", self.id).into());
            let preview_id = ElementId::Name(format!("{}-preview", self.id).into());

            let preview_el = if summary.is_hr {
                div()
                    .flex_1()
                    .h(px(2.))
                    .rounded_full()
                    .bg(muted_fg.opacity(0.35))
                    .into_any_element()
            } else {
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        Label::new(summary.text)
                            .id(preview_id)
                            .text_xs()
                            .truncate()
                            .when(summary.is_muted, |l| {
                                l.text_color(muted_fg.opacity(0.75)).italic()
                            })
                            .when(!summary.is_muted, |l| l.text_color(theme.foreground)),
                    )
                    .into_any_element()
            };

            return h_flex()
                .id(self.id.clone())
                .w_full()
                .min_w_0()
                .items_center()
                .gap_2()
                .px_3()
                .py_1p5()
                .rounded_md()
                .bg(bg)
                .border_1()
                .when(self.is_dragging_source, |el| {
                    el.border_dashed()
                        .border_color(theme.primary.opacity(0.6))
                        .opacity(0.45)
                })
                .when(!self.is_dragging_source, |el| el.border_color(border_color))
                .when(self.show_drag_handle, |el| {
                    el.child(
                        div()
                            .flex_shrink_0()
                            .text_color(muted_fg.opacity(0.55))
                            .child(Icon::new(IconName::GripVertical).xsmall()),
                    )
                })
                .child(
                    div()
                        .flex_shrink_0()
                        .px_1p5()
                        .py_0p5()
                        .rounded_sm()
                        .bg(badge_bg)
                        .child(
                            Label::new(self.badge)
                                .id(badge_id)
                                .text_xs()
                                .font_semibold()
                                .text_color(muted_fg),
                        ),
                )
                .child(preview_el)
                .into_any_element();
        }

        let drag_handle_id = ElementId::Name(format!("{}-drag-handle", self.id).into());
        let badge_id = ElementId::Name(format!("{}-badge", self.id).into());
        let mut drag_handle = h_flex()
            .id(drag_handle_id)
            .flex_1()
            .min_w_0()
            .items_center()
            .gap_1p5()
            .when(self.show_drag_handle, |el| {
                el.cursor_grab().child(
                    div()
                        .text_color(muted_fg.opacity(0.55))
                        .hover(|s| s.text_color(muted_fg))
                        .child(Icon::new(IconName::GripVertical).xsmall()),
                )
            })
            .child(
                Label::new(self.badge)
                    .id(badge_id)
                    .text_xs()
                    .font_medium()
                    .text_color(muted_fg.opacity(0.75)),
            );

        if let Some(on_drag) = self.on_drag {
            drag_handle = on_drag(drag_handle);
        }

        v_flex()
            .id(self.id.clone())
            .w_full()
            .rounded_md()
            .bg(bg)
            .border_1()
            .border_color(border_color)
            .px_3()
            .py_2()
            .gap_1p5()
            // Compact, low-contrast header bar
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_1p5()
                    .child(drag_handle)
                    .when(self.show_remove, |el| {
                        let remove_id = ElementId::Name(format!("{}-remove", self.id).into());
                        let on_remove = self.on_remove.clone();
                        el.child(
                            div()
                                .opacity(0.4)
                                .hover(|s| s.opacity(1.0))
                                .child(
                                    Button::new(remove_id)
                                        .xsmall()
                                        .ghost()
                                        .icon(Icon::new(IconName::Trash))
                                        .on_click(move |ev, window, cx| {
                                            if let Some(cb) = &on_remove {
                                                cb(ev, window, cx);
                                            }
                                        }),
                                ),
                        )
                    }),
            )
            // Content slot
            .children(self.children)
            .into_any_element()
    }
}

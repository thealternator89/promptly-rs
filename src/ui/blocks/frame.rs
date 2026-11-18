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

/// Shared container frame for both top-level and nested blocks.
#[derive(IntoElement)]
pub struct BlockFrame {
    id: ElementId,
    badge: &'static str,
    is_nested: bool,
    show_drag_handle: bool,
    show_remove: bool,
    on_remove: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
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
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1p5()
                            .when(self.show_drag_handle, |el| {
                                el.child(
                                    div()
                                        .cursor_grab()
                                        .text_color(muted_fg.opacity(0.55))
                                        .hover(|s| s.text_color(muted_fg))
                                        .child(Icon::new(IconName::GripVertical).xsmall()),
                                )
                            })
                            .child(
                                Label::new(self.badge)
                                    .text_xs()
                                    .font_medium()
                                    .text_color(muted_fg.opacity(0.75)),
                            ),
                    )
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
    }
}

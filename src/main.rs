pub mod app;
pub mod model;
pub mod services;
pub mod ui;

use gpui_kit::component::TitleBar;
use gpui_kit::*;

use crate::app::AppShell;

fn main() {
    application()
        .with_assets(assets::AllAssets)
        .run(|cx| {
            init(cx);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1366.), px(768.)),
                    cx,
                ))),
                ..TitleBar::window_options()
            };

            open_window(options, cx, |window, cx| {
                cx.new(|cx| AppShell::new(window, cx))
            })
            .expect("Failed to open window");
        });
}

#![windows_subsystem = "windows"]

pub mod app;
pub mod branding;
pub mod model;
pub mod services;
pub mod ui;
pub mod version;

use gpui_kit::component::TitleBar;
use gpui_kit::*;

use crate::app::AppShell;
use crate::branding::AppAssets;

fn main() {
    application()
        .with_assets(AppAssets)
        .run(|cx| {
            init(cx);
            branding::set_app_dock_icon();

            let mut titlebar = TitleBar::title_bar_options();
            titlebar.title = Some("Promptly".into());

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1366.), px(768.)),
                    cx,
                ))),
                titlebar: Some(titlebar),
                app_id: Some("Promptly".to_string()),
                ..TitleBar::window_options()
            };

            open_window(options, cx, |window, cx| {
                cx.new(|cx| AppShell::new(window, cx))
            })
            .expect("Failed to open window");
        });
}

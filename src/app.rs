use crate::assets::{app_window_icon, MAIN_CSS, TAILWIND_CSS};
use crate::components::TodoApp;
use crate::config::{APP_WINDOW_HEIGHT, APP_WINDOW_WIDTH};
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

pub(crate) fn run() {
    let window = WindowBuilder::new()
        .with_title("So Todo")
        .with_inner_size(LogicalSize::new(APP_WINDOW_WIDTH, APP_WINDOW_HEIGHT))
        .with_min_inner_size(LogicalSize::new(360.0, 560.0))
        .with_decorations(false);

    let mut config = Config::new().with_window(window);
    if let Some(icon) = app_window_icon() {
        config = config.with_icon(icon);
    }

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(App);
}

#[component]
pub(crate) fn App() -> Element {
    rsx! {
        style { "{TAILWIND_CSS}\n{MAIN_CSS}" }
        TodoApp {}
    }
}

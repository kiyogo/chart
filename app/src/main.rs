fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
        const SCROLLBARS_JS: &str = include_str!("../web/canvas_scrollbars.js");
        const RESIZER_JS: &str = include_str!("../web/panel_resizer.js");
        const STORAGE_JS: &str = include_str!("../web/storage_engine.js");
        const MEDIA_JS: &str = include_str!("../web/media_optimizer.js");
        const PLUGIN_RUNTIME_JS: &str = include_str!("../web/plugin_runtime.js");

        let custom_head = format!(
            "<script>{SCROLLBARS_JS}</script>\n<script>{RESIZER_JS}</script>\n<script>{STORAGE_JS}</script>\n<script>{MEDIA_JS}</script>\n<script>{PLUGIN_RUNTIME_JS}</script>"
        );
        let window = WindowBuilder::new()
            .with_title("ChatW")
            .with_inner_size(LogicalSize::new(1280.0, 800.0))
            .with_min_inner_size(LogicalSize::new(840.0, 560.0));
        dioxus::LaunchBuilder::desktop()
            .with_cfg(
                Config::new()
                    .with_window(window)
                    .with_menu(None)
                    .with_custom_head(custom_head),
            )
            .launch(chatw_app::App);
    }

    #[cfg(all(not(feature = "desktop"), any(feature = "web", feature = "mobile")))]
    {
        dioxus::launch(chatw_app::App);
    }

    #[cfg(not(any(feature = "web", feature = "desktop", feature = "mobile")))]
    {
        eprintln!("Enable one of --features web, --features desktop, or --features mobile");
    }
}


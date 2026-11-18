fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
        let window = WindowBuilder::new()
            .with_title("chatw - Ứng dụng nhắn tin bảo mật mã hóa đầu cuối E2EE")
            .with_inner_size(LogicalSize::new(1280.0, 800.0));
        dioxus::LaunchBuilder::desktop()
            .with_cfg(Config::new().with_window(window))
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


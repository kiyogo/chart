#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod net;
mod storage;
mod titlebar;
mod views;

use app::ChatApp;
use eframe::egui::ViewportBuilder;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("chatw")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([850.0, 560.0])
            .with_decorations(false) // Seamless custom Windows Titlebar
            .with_transparent(false),
        ..Default::default()
    };

    eframe::run_native(
        "chatw",
        options,
        Box::new(|cc| Ok(Box::new(ChatApp::new(cc)))),
    )
}

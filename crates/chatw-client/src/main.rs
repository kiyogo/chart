#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod net;
mod storage;
mod titlebar;
mod views;

use app::ChatApp;
use eframe::egui::ViewportBuilder;

fn log_app(msg: &str) {
    let line = format!("[{}] {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), msg);
    let _ = std::fs::OpenOptions::new().create(true).append(true).open("app_debug.log").map(|mut f| {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    });
}

fn main() -> eframe::Result<()> {
    log_app("chatw main started");

    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::capture();
        let msg = format!("CHATW CLIENT PANIC CRASH:\n{:#?}\n\nBacktrace:\n{:?}\n", info, backtrace);
        eprintln!("{}", msg);
        log_app(&msg);
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let path = std::path::PathBuf::from(app_data).join("chatw").join("crash.log");
            let _ = std::fs::write(path, &msg);
        }
        let _ = std::fs::write("chatw_crash.log", &msg);
    }));

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("chatw")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([850.0, 560.0])
            .with_decorations(false) // Seamless custom Windows Titlebar
            .with_transparent(false),
        ..Default::default()
    };

    log_app("Calling eframe::run_native");

    let result = eframe::run_native(
        "chatw",
        options,
        Box::new(|cc| {
            log_app("Inside eframe App creator closure");
            let app = ChatApp::new(cc);
            log_app("ChatApp created successfully");
            Ok(Box::new(app))
        }),
    );

    log_app(&format!("eframe::run_native exited with result: {:?}", result.as_ref().map(|_| "Ok").map_err(|e| e.to_string())));

    if let Err(ref e) = result {
        let msg = format!("CHATW RUN_NATIVE ERROR:\n{:#?}\n", e);
        eprintln!("{}", msg);
        log_app(&msg);
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let path = std::path::PathBuf::from(app_data).join("chatw").join("crash.log");
            let _ = std::fs::write(path, &msg);
        }
        let _ = std::fs::write("chatw_crash.log", &msg);
    }

    result
}

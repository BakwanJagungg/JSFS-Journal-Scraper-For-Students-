#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use jsfs::config;
use jsfs::ui::JsfsNativeApp;
use std::sync::Arc;

fn init_tracing() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let dir = config::logs_dir();
    if std::fs::create_dir_all(&dir).is_ok() {
        let file_appender = tracing_appender::rolling::daily(&dir, "jsfs.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        let _ = tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_ansi(false)
            .try_init();
        return Some(guard);
    }
    None
}

fn load_app_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/icon.ico");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let _log_guard = init_tracing();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!(
            "JSFS v{} — Journal Scraper For Students",
            config::APP_VERSION
        ))
        .with_inner_size([1240.0, 800.0])
        .with_min_inner_size([920.0, 620.0]);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "JSFS",
        options,
        Box::new(|cc| Ok(Box::new(JsfsNativeApp::new(cc)))),
    )
}

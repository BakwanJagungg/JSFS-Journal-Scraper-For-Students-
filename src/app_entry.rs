//! Shared desktop application launcher used by both `jsfs` (Standard) and `jsfs-plus` (Workstation).

use crate::config;
use crate::ui::JsfsNativeApp;
use std::sync::Arc;

#[cfg(target_os = "windows")]
fn show_native_error_dialog(title: &str, msg: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let title_wide: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();
    let msg_wide: Vec<u16> = OsStr::new(msg).encode_wide().chain(Some(0)).collect();

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            hwnd: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            utype: u32,
        ) -> i32;
    }

    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            msg_wide.as_ptr(),
            title_wide.as_ptr(),
            0x0000_0010, // MB_ICONERROR | MB_OK
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_native_error_dialog(title: &str, msg: &str) {
    eprintln!("[FATAL ERROR] {title}: {msg}");
}

fn write_startup_error_log(context: &str, details: &str) {
    let cache_dir = config::cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);
    let log_path = cache_dir.join("startup_error.log");
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let entry =
        format!("[{timestamp}] {context}\n{details}\n----------------------------------------\n");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
}

fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Kesalahan tidak teridentifikasi".to_string()
        };
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "lokasi tidak diketahui".to_string());

        let log_msg = format!("Panic di {location}: {payload}\nInfo lengkap: {panic_info}");
        write_startup_error_log("CRITICAL PANIC", &log_msg);

        let dialog_msg = format!(
            "JSFS mengalami kesalahan kritis saat berjalan:\n\n\
             Lokasi: {location}\n\
             Pesan: {payload}\n\n\
             Detail kesalahan telah dicatat di:\n\
             .cache/startup_error.log\n\n\
             Silakan laporkan kendala ini kepada pengembang."
        );
        show_native_error_dialog("JSFS - Kesalahan Kritis", &dialog_msg);
    }));
}

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

#[cfg(target_os = "windows")]
fn initial_viewport_size() -> [f32; 2] {
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn SystemParametersInfoW(
            ui_action: u32,
            ui_param: u32,
            pv_param: *mut Rect,
            f_win_ini: u32,
        ) -> i32;
        fn GetDpiForSystem() -> u32;
    }

    let mut rc = Rect {
        left: 0,
        top: 0,
        right: 1280,
        bottom: 800,
    };
    let ok = unsafe { SystemParametersInfoW(0x0030, 0, &mut rc, 0) }; // SPI_GETWORKAREA
    if ok != 0 {
        let dpi = unsafe { GetDpiForSystem() }.max(96) as f32;
        let scale = dpi / 96.0;
        let work_w = ((rc.right - rc.left) as f32 / scale - 32.0).max(820.0);
        let work_h = ((rc.bottom - rc.top) as f32 / scale - 36.0).max(500.0);
        return [1240.0_f32.min(work_w), 800.0_f32.min(work_h)];
    }
    [1240.0, 800.0]
}

#[cfg(not(target_os = "windows"))]
fn initial_viewport_size() -> [f32; 2] {
    [1180.0, 760.0]
}

/// Initialize panic hook, logging, viewport, and run native `eframe` desktop loop.
pub fn run_desktop_app() -> eframe::Result<()> {
    setup_panic_hook();
    crate::engine::updater::cleanup_old_updater_binary();
    config::ensure_runtime_dirs();
    let _log_guard = init_tracing();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!(
            "{} v{} - Journal Scraper For Students",
            config::APP_EDITION,
            config::APP_VERSION
        ))
        .with_inner_size(initial_viewport_size())
        .with_min_inner_size([820.0, 500.0]);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let result = eframe::run_native(
        config::APP_EDITION,
        options,
        Box::new(|cc| Ok(Box::new(JsfsNativeApp::new(cc)))),
    );

    if let Err(ref e) = result {
        let err_str = format!("{e}");
        write_startup_error_log("EFRAME RUN NATIVE ERROR", &err_str);
        let hint = if cfg!(target_os = "windows") {
            "Pastikan driver grafis mendukung hardware acceleration (DirectX / OpenGL)."
        } else {
            "Pastikan sesi desktop (Wayland / X11 / Metal) dan driver OpenGL aktif."
        };
        let dialog_msg = format!(
            "Gagal membuka jendela utama JSFS:\n\n\
             {err_str}\n\n\
             {hint}\n\
             Log telah disimpan di .cache/startup_error.log."
        );
        show_native_error_dialog("JSFS - Gagal Memulai GUI", &dialog_msg);
    }

    result
}

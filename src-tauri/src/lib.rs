//! JSFS v3.0.0 — Journal Scraper For Students (Rust + Tauri v2)

pub mod ai;
pub mod cache;
pub mod commands;
pub mod config;
pub mod error;
pub mod exporter;
pub mod models;
pub mod parser;
pub mod scraper;

use std::sync::{atomic::AtomicBool, Arc, Mutex};
use tauri::Manager;
use tokio::sync::RwLock;

/// Shared application state managed by Tauri across all async commands.
pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
    pub is_running: Arc<AtomicBool>,
    pub current_results: Arc<RwLock<Vec<models::JournalRecord>>>,
    pub manuscript: Arc<RwLock<Option<models::ManuscriptMeta>>>,
    pub last_excel_path: Arc<RwLock<Option<String>>>,
    pub cache: Arc<Mutex<Option<cache::JournalCache>>>,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new() -> Self {
        let http_client = scraper::build_http_client();
        let db = cache::JournalCache::open_default().ok();
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
            current_results: Arc::new(RwLock::new(Vec::new())),
            manuscript: Arc::new(RwLock::new(None)),
            last_excel_path: Arc::new(RwLock::new(None)),
            cache: Arc::new(Mutex::new(db)),
            http_client,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
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

/// Entry point called by `main.rs`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _log_guard = init_tracing();
    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::search::start_search,
            commands::search::stop_search,
            commands::manuscript::upload_manuscript,
            commands::ai_ops::get_ai_status,
            commands::ai_ops::launch_ollama,
            commands::ai_ops::install_ollama,
            commands::ai_ops::pull_ai_model,
            commands::file_ops::open_excel,
            commands::file_ops::open_output_folder,
            commands::file_ops::open_url,
            commands::file_ops::open_logs_folder,
            commands::cache_ops::clear_cache,
            commands::file_ops::copy_share_list,
            commands::file_ops::copy_editor_draft,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let state = window.state::<AppState>();
                if let Ok(guard) = state.cache.lock() {
                    if let Some(ref db) = *guard {
                        db.checkpoint_wal();
                    }
                };
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running JSFS v3 application");
}

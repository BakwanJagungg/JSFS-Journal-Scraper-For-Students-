use std::path::PathBuf;

pub const APP_VERSION: &str = "3.0.0";
pub const CACHE_VERSION: &str = "3.0.0";
pub const CACHE_TTL_DAYS: i64 = 14;
pub const MAX_WORKERS: usize = 32;
pub const OJS_HARD_TIMEOUT_MS: u64 = 5500;
pub const OLLAMA_PHASE_BUDGET_SECS: u64 = 45;
pub const OLLAMA_MAX_ITEMS: usize = 8;
pub const OLLAMA_PER_ITEM_TIMEOUT_SECS: u64 = 5;
pub const AUTOSAVE_INTERVAL_ITEMS: usize = 50;
pub const MAX_HTML_BYTES: usize = 1_572_864; // 1.5 MB
pub const MAX_TARGET_COUNT: usize = 1000;
pub const DEFAULT_AI_MODEL: &str = "qwen2.5:1.5b";
pub const OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";

pub const GARUDA_BASE_URL: &str = "https://garuda.kemdiktisaintek.go.id";
pub const GARUDA_JOURNAL_URL: &str = "https://garuda.kemdiktisaintek.go.id/journal";
pub const SINTA_BASE_URL: &str = "https://sinta.kemdiktisaintek.go.id";
pub const SINTA_JOURNAL_URL: &str = "https://sinta.kemdiktisaintek.go.id/journals";

pub const INFO_NOT_FOUND: &str = "Informasi tidak ditemukan dalam situs jurnal";
pub const DEAD_LINK_TEXT: &str = "Situs tidak dapat diakses";
pub const TIMEOUT_APC_LABEL: &str = "Cek Web / Timeout";
pub const APC_UNKNOWN: &str = "Tidak dicantumkan (Cek Web)";

pub const USD_TO_IDR_RATE: i64 = 17_500;
pub const APC_SANITY_MIN_IDR: i64 = 50_000;
pub const APC_SANITY_MAX_IDR: i64 = 15_000_000;

/// Resolve the base directory for JSFS data (`.cache/`, `output/`, `logs/`).
///
/// Follows Pantangan V3-3: stored alongside the executable (`current_exe().parent()`).
/// Never writes persistent caches or outputs into `%TEMP%`.
pub fn app_base_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let dir_str = dir.to_string_lossy();
            if !dir_str.contains("target\\debug")
                && !dir_str.contains("target/debug")
                && !dir_str.contains("target\\release")
                && !dir_str.contains("target/release")
            {
                return dir.to_path_buf();
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn cache_dir() -> PathBuf {
    app_base_dir().join(".cache")
}

pub fn cache_db_path() -> PathBuf {
    cache_dir().join("jsfs_cache.db")
}

pub fn output_dir() -> PathBuf {
    app_base_dir().join("output")
}

pub fn logs_dir() -> PathBuf {
    app_base_dir().join("logs")
}

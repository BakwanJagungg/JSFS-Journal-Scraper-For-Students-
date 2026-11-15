use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const APP_VERSION: &str = "4.3.0";
pub const CACHE_VERSION: &str = "4.3.0";
pub const CACHE_TTL_DAYS: i64 = 14;
pub const MAX_WORKERS: usize = 32;
pub const LOW_END_MAX_WORKERS: usize = 14;
pub const LOW_END_CPU_THRESHOLD: usize = 4;
pub const SQLITE_CACHE_SIZE_KIB: i64 = 4000;
pub const OJS_HARD_TIMEOUT_MS: u64 = 5500;
pub const OLLAMA_PHASE_BUDGET_SECS: u64 = 45;
pub const OLLAMA_MAX_ITEMS: usize = 8;
pub const OLLAMA_PER_ITEM_TIMEOUT_SECS: u64 = 5;
pub const CLOUD_PER_ITEM_TIMEOUT_SECS: u64 = 8;
pub const AUTOSAVE_INTERVAL_ITEMS: usize = 50;
pub const MAX_HTML_BYTES: usize = 1_572_864; // 1.5 MB
pub const MAX_TARGET_COUNT: usize = 1000;
pub const DEFAULT_AI_MODEL: &str = "qwen2.5:1.5b";
pub const OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";
pub const DEFAULT_GROQ_MODEL: &str = "llama-3.1-8b-instant";
pub const DEFAULT_GEMINI_MODEL: &str = "gemini-2.5-flash";
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";

pub const GARUDA_BASE_URL: &str = "https://garuda.kemdiktisaintek.go.id";
pub const GARUDA_JOURNAL_URL: &str = "https://garuda.kemdiktisaintek.go.id/journal";
pub const SINTA_BASE_URL: &str = "https://sinta.kemdiktisaintek.go.id";
pub const SINTA_JOURNAL_URL: &str = "https://sinta.kemdiktisaintek.go.id/journals";

pub const INFO_NOT_FOUND: &str = "Informasi tidak ditemukan dalam situs jurnal";
pub const DEAD_LINK_TEXT: &str = "Situs tidak dapat diakses";
pub const TIMEOUT_APC_LABEL: &str = "Cek Web / Timeout";
pub const APC_UNKNOWN: &str = "Tidak dicantumkan (Cek Web)";

pub const USD_TO_IDR_RATE: i64 = 16_500;
pub const APC_SANITY_MIN_IDR: i64 = 50_000;
pub const APC_SANITY_MAX_IDR: i64 = 15_000_000;

static RESOLVED_BASE_DIR: OnceCell<PathBuf> = OnceCell::new();

/// Number of logical CPUs available to this process (falls back to 1).
pub fn logical_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Deep-crawl concurrency scaled to the machine: 14 on <= 4 logical CPUs, otherwise 32.
pub fn workers_for_cpus(cpus: usize) -> usize {
    if cpus <= LOW_END_CPU_THRESHOLD {
        LOW_END_MAX_WORKERS
    } else {
        MAX_WORKERS
    }
}

pub fn effective_max_workers() -> usize {
    static WORKERS: OnceCell<usize> = OnceCell::new();
    *WORKERS.get_or_init(|| workers_for_cpus(logical_cpus()))
}

fn is_dir_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(format!(".jsfs_write_probe_{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn local_appdata_jsfs_dir() -> Option<PathBuf> {
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let p = PathBuf::from(local).join("JSFS");
        if !p.as_os_str().is_empty() {
            return Some(p);
        }
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        return Some(
            PathBuf::from(home)
                .join("AppData")
                .join("Local")
                .join("JSFS"),
        );
    }
    None
}

fn resolve_writable_base_dir(preferred: PathBuf, fallback: Option<PathBuf>) -> PathBuf {
    if is_dir_writable(&preferred) {
        return preferred;
    }
    if let Some(fb) = fallback {
        if is_dir_writable(&fb) {
            return fb;
        }
        return fb;
    }
    preferred
}

fn preferred_base_dir() -> PathBuf {
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

/// Resolve the base directory for JSFS data (`.cache/`, `output/`, `logs/`).
///
/// Prefers the directory alongside the executable (`current_exe().parent()`) for portable usage,
/// and automatically falls back to `%LOCALAPPDATA%\JSFS` if that folder is read-only or blocked
/// by Windows Controlled Folder Access.
pub fn app_base_dir() -> PathBuf {
    RESOLVED_BASE_DIR
        .get_or_init(|| resolve_writable_base_dir(preferred_base_dir(), local_appdata_jsfs_dir()))
        .clone()
}

/// Ensure runtime directories (`.cache/`, `output/`, `logs/`) exist without panicking.
pub fn ensure_runtime_dirs() {
    for dir in [cache_dir(), output_dir(), logs_dir()] {
        let _ = std::fs::create_dir_all(dir);
    }
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

pub fn matcher_config_path() -> PathBuf {
    cache_dir().join("matcher_config.json")
}

pub fn search_history_path() -> PathBuf {
    cache_dir().join("search_history.json")
}

pub fn app_settings_path() -> PathBuf {
    cache_dir().join("app_settings.json")
}

/// Persisted user preference for JSFS Match Engine (`builtin` | `cloud` | `ollama`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MatcherConfig {
    pub mode: String,
    pub cloud_provider: String,
    pub cloud_api_key: String,
    pub cloud_model: String,
    pub cloud_base_url: String,
    pub ollama_model: String,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            mode: "builtin".to_string(),
            cloud_provider: "groq".to_string(),
            cloud_api_key: String::new(),
            cloud_model: DEFAULT_GROQ_MODEL.to_string(),
            cloud_base_url: "https://api.openai.com/v1".to_string(),
            ollama_model: DEFAULT_AI_MODEL.to_string(),
        }
    }
}

impl MatcherConfig {
    pub fn normalized_mode(&self) -> &str {
        match self.mode.trim().to_ascii_lowercase().as_str() {
            "cloud" => "cloud",
            "ollama" => "ollama",
            _ => "builtin",
        }
    }

    pub fn normalized_provider(&self) -> &str {
        match self.cloud_provider.trim().to_ascii_lowercase().as_str() {
            "gemini" => "gemini",
            "openai" | "openai_compatible" | "openai-compatible" => "openai_compatible",
            _ => "groq",
        }
    }

    pub fn effective_cloud_model(&self) -> String {
        let m = self.cloud_model.trim();
        if !m.is_empty() {
            return m.to_string();
        }
        default_model_for_provider(self.normalized_provider()).to_string()
    }
}

pub fn default_model_for_provider(provider: &str) -> &'static str {
    match provider {
        "gemini" => DEFAULT_GEMINI_MODEL,
        "openai" | "openai_compatible" | "openai-compatible" => DEFAULT_OPENAI_MODEL,
        _ => DEFAULT_GROQ_MODEL,
    }
}

pub fn mask_api_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix: String = chars.iter().take(4).collect();
    let suffix: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{prefix}••••{suffix}")
}

pub fn load_matcher_config() -> MatcherConfig {
    let path = matcher_config_path();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(mut cfg) = serde_json::from_str::<MatcherConfig>(&raw) {
            cfg.mode = cfg.normalized_mode().to_string();
            cfg.cloud_provider = cfg.normalized_provider().to_string();
            return cfg;
        }
    }
    MatcherConfig::default()
}

pub fn save_matcher_config_to_disk(cfg: &MatcherConfig) -> std::io::Result<()> {
    let path = matcher_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_end_worker_scaling() {
        assert_eq!(workers_for_cpus(1), LOW_END_MAX_WORKERS);
        assert_eq!(workers_for_cpus(4), LOW_END_MAX_WORKERS);
        assert_eq!(workers_for_cpus(8), MAX_WORKERS);
        assert!(effective_max_workers() >= LOW_END_MAX_WORKERS);
    }

    #[test]
    fn test_writable_base_dir_fallback() {
        let writable = std::env::temp_dir().join(format!("jsfs_cfg_ok_{}", std::process::id()));
        let fallback = std::env::temp_dir().join(format!("jsfs_cfg_fb_{}", std::process::id()));

        let chosen = resolve_writable_base_dir(writable.clone(), Some(fallback.clone()));
        assert_eq!(chosen, writable);

        let blocked_pref =
            std::env::temp_dir().join(format!("jsfs_cfg_file_{}", std::process::id()));
        std::fs::write(&blocked_pref, b"not a directory").expect("create dummy file");
        let chosen_fb = resolve_writable_base_dir(blocked_pref.clone(), Some(fallback.clone()));
        assert_eq!(chosen_fb, fallback);

        let _ = std::fs::remove_dir_all(&writable);
        let _ = std::fs::remove_dir_all(&fallback);
        let _ = std::fs::remove_file(&blocked_pref);
    }
}

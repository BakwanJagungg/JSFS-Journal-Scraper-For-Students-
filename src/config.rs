use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const APP_VERSION: &str = "4.0.1";
pub const CACHE_VERSION: &str = "4.0.1";
pub const CACHE_TTL_DAYS: i64 = 14;
pub const MAX_WORKERS: usize = 32;
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

pub fn matcher_config_path() -> PathBuf {
    cache_dir().join("matcher_config.json")
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

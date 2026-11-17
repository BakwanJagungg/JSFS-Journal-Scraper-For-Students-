use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const APP_VERSION: &str = "4.12.1";
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
static RESOLVED_OUTPUT_DIR: OnceCell<PathBuf> = OnceCell::new();

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

fn documents_jsfs_output_dir() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let p = PathBuf::from(home).join("Documents").join("JSFS Output");
        if !p.as_os_str().is_empty() {
            return Some(p);
        }
    }
    None
}

fn is_installed_location(dir: &Path) -> bool {
    let lower = dir
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    lower.contains("\\appdata\\local\\programs\\jsfs")
        || lower.contains("\\program files\\")
        || lower.contains("\\program files (x86)\\")
        || lower.ends_with("\\program files")
}

fn resolve_output_dir(
    exe_or_pref_dir: &Path,
    portable_base: &Path,
    docs_out: Option<PathBuf>,
    local_appdata_base: Option<PathBuf>,
) -> PathBuf {
    let local_out = local_appdata_base.map(|b| b.join("output"));
    if is_installed_location(exe_or_pref_dir) {
        if let Some(ref docs) = docs_out {
            if is_dir_writable(docs) {
                return docs.clone();
            }
        }
        if let Some(ref fb) = local_out {
            if is_dir_writable(fb) {
                return fb.clone();
            }
            return fb.clone();
        }
    }

    let portable_out = portable_base.join("output");
    if is_dir_writable(exe_or_pref_dir) && is_dir_writable(&portable_out) {
        return portable_out;
    }
    if let Some(ref docs) = docs_out {
        if is_dir_writable(docs) {
            return docs.clone();
        }
    }
    if let Some(fb) = local_out {
        if is_dir_writable(&fb) {
            return fb;
        }
        return fb;
    }
    portable_out
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

/// Resolve the output directory for exported reports (`.xlsx`, `.md`, `.csv`, `_LIST.txt`).
///
/// Installed builds (under `AppData\Local\Programs\JSFS` or `Program Files`) write to
/// `%USERPROFILE%\Documents\JSFS Output` (falling back to `%LOCALAPPDATA%\JSFS\output`) so users
/// can find reports easily. Portable runs use `./output` beside `jsfs.exe`, falling back to
/// `Documents\JSFS Output` if read-only.
pub fn output_dir() -> PathBuf {
    RESOLVED_OUTPUT_DIR
        .get_or_init(|| {
            let pref = preferred_base_dir();
            let base = app_base_dir();
            resolve_output_dir(
                &pref,
                &base,
                documents_jsfs_output_dir(),
                local_appdata_jsfs_dir(),
            )
        })
        .clone()
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

pub fn saved_profiles_path() -> PathBuf {
    cache_dir().join("saved_profiles.json")
}

pub fn saved_manuscripts_path() -> PathBuf {
    cache_dir().join("saved_manuscripts.json")
}

pub fn saved_shortlist_path() -> PathBuf {
    cache_dir().join("saved_shortlist.json")
}

/// Load saved shortlisted journals (`★`) from disk, returning empty on missing/corrupt files.
pub fn load_saved_shortlist_from(path: &Path) -> Vec<crate::models::UiJournalItem> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn load_saved_shortlist() -> Vec<crate::models::UiJournalItem> {
    load_saved_shortlist_from(&saved_shortlist_path())
}

/// Persist saved shortlisted journals (`★`) atomically-enough via pretty JSON.
pub fn save_saved_shortlist_to(
    path: &Path,
    items: &[crate::models::UiJournalItem],
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(items)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

pub fn save_saved_shortlist(items: &[crate::models::UiJournalItem]) -> std::io::Result<()> {
    save_saved_shortlist_to(&saved_shortlist_path(), items)
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

pub const MAX_SAVED_MANUSCRIPTS: usize = 5;

/// Lightweight metadata summary for switching among recently loaded manuscripts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedManuscriptEntry {
    pub file_name: String,
    pub file_path: String,
    pub word_count: usize,
    pub extracted_title: String,
    pub suggested_keywords: String,
}

/// Load saved manuscript summaries from disk, returning empty on missing/corrupt files.
pub fn load_saved_manuscripts_from(path: &Path) -> Vec<SavedManuscriptEntry> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut entries: Vec<SavedManuscriptEntry> = serde_json::from_str(&raw).unwrap_or_default();
    entries.truncate(MAX_SAVED_MANUSCRIPTS);
    entries
}

pub fn load_saved_manuscripts() -> Vec<SavedManuscriptEntry> {
    load_saved_manuscripts_from(&saved_manuscripts_path())
}

/// Persist saved manuscript summaries atomically-enough via pretty JSON.
pub fn save_saved_manuscripts_to(
    path: &Path,
    entries: &[SavedManuscriptEntry],
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let capped: Vec<&SavedManuscriptEntry> = entries.iter().take(MAX_SAVED_MANUSCRIPTS).collect();
    let json = serde_json::to_string_pretty(&capped)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

pub fn save_saved_manuscripts(entries: &[SavedManuscriptEntry]) -> std::io::Result<()> {
    save_saved_manuscripts_to(&saved_manuscripts_path(), entries)
}

/// Insert or move `entry` to the front, deduplicating by path and capping at `MAX_SAVED_MANUSCRIPTS`.
pub fn record_saved_manuscript(
    entries: &mut Vec<SavedManuscriptEntry>,
    entry: SavedManuscriptEntry,
) {
    if entry.file_path.trim().is_empty() {
        return;
    }
    entries.retain(|item| !item.file_path.eq_ignore_ascii_case(&entry.file_path));
    entries.insert(0, entry);
    entries.truncate(MAX_SAVED_MANUSCRIPTS);
}

/// Portable backup bundle combining all user configurations, profiles, saved manuscripts, shortlisted journals, and submission tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsfsBackupBundle {
    pub bundle_version: String,
    pub jsfs_version: String,
    pub exported_at: String,
    pub settings: serde_json::Value,
    pub saved_profiles: serde_json::Value,
    pub saved_manuscripts: Vec<SavedManuscriptEntry>,
    pub shortlisted_keys: Vec<String>,
    #[serde(default)]
    pub saved_shortlist: Vec<crate::models::UiJournalItem>,
    pub submission_tracks: Vec<crate::cache::SubmissionTrack>,
}

impl JsfsBackupBundle {
    pub fn new(
        settings_val: serde_json::Value,
        profiles_val: serde_json::Value,
        manuscripts: Vec<SavedManuscriptEntry>,
        shortlisted: Vec<String>,
        saved_shortlist: Vec<crate::models::UiJournalItem>,
        tracks: Vec<crate::cache::SubmissionTrack>,
    ) -> Self {
        Self {
            bundle_version: "1.0".to_string(),
            jsfs_version: APP_VERSION.to_string(),
            exported_at: chrono::Local::now().to_rfc3339(),
            settings: settings_val,
            saved_profiles: profiles_val,
            saved_manuscripts: manuscripts,
            shortlisted_keys: shortlisted,
            saved_shortlist,
            submission_tracks: tracks,
        }
    }

    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json_str(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }
}

pub fn create_backup_bundle(
    settings_val: serde_json::Value,
    profiles_val: serde_json::Value,
    shortlisted: Vec<String>,
    saved_shortlist: Vec<crate::models::UiJournalItem>,
    tracks: Vec<crate::cache::SubmissionTrack>,
) -> JsfsBackupBundle {
    let manuscripts = load_saved_manuscripts();
    JsfsBackupBundle::new(
        settings_val,
        profiles_val,
        manuscripts,
        shortlisted,
        saved_shortlist,
        tracks,
    )
}

pub fn save_backup_bundle_to_path(bundle: &JsfsBackupBundle, dest: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = bundle
        .to_json_pretty()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(dest, json)
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

    #[test]
    fn test_installed_and_portable_output_dir_resolution() {
        assert!(is_installed_location(Path::new(
            r"C:\Users\Student\AppData\Local\Programs\JSFS"
        )));
        assert!(is_installed_location(Path::new(r"C:\Program Files\JSFS")));
        assert!(!is_installed_location(Path::new(r"D:\PortableApps\JSFS")));

        let pid = std::process::id();
        let docs_out = std::env::temp_dir().join(format!("jsfs_docs_out_{pid}"));
        let local_fb = std::env::temp_dir().join(format!("jsfs_local_fb_{pid}"));
        let portable = std::env::temp_dir().join(format!("jsfs_port_dir_{pid}"));
        let blocked = std::env::temp_dir().join(format!("jsfs_blocked_file_{pid}"));
        std::fs::write(&blocked, b"file").expect("dummy");

        let installed_out = resolve_output_dir(
            Path::new(r"C:\Users\Student\AppData\Local\Programs\JSFS"),
            &local_fb,
            Some(docs_out.clone()),
            Some(local_fb.clone()),
        );
        assert_eq!(installed_out, docs_out);

        let port_out = resolve_output_dir(
            &portable,
            &portable,
            Some(docs_out.clone()),
            Some(local_fb.clone()),
        );
        assert_eq!(port_out, portable.join("output"));

        let ro_port_out = resolve_output_dir(
            &blocked,
            &local_fb,
            Some(docs_out.clone()),
            Some(local_fb.clone()),
        );
        assert_eq!(ro_port_out, docs_out);

        let _ = std::fs::remove_dir_all(&docs_out);
        let _ = std::fs::remove_dir_all(&local_fb);
        let _ = std::fs::remove_dir_all(&portable);
        let _ = std::fs::remove_file(&blocked);
    }

    #[test]
    fn test_saved_manuscripts_persistence() {
        let path =
            std::env::temp_dir().join(format!("jsfs_saved_ms_test_{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let mut list = Vec::new();
        for i in 1..=6 {
            record_saved_manuscript(
                &mut list,
                SavedManuscriptEntry {
                    file_name: format!("paper_{i}.docx"),
                    file_path: format!(r"C:\docs\paper_{i}.docx"),
                    word_count: 3000 + i * 100,
                    extracted_title: format!("Title {i}"),
                    suggested_keywords: format!("kw{i}"),
                },
            );
        }
        assert_eq!(list.len(), MAX_SAVED_MANUSCRIPTS);
        assert_eq!(list[0].file_name, "paper_6.docx");
        assert_eq!(list[4].file_name, "paper_2.docx");

        // Re-recording an older path moves it back to the front without duplicates.
        record_saved_manuscript(
            &mut list,
            SavedManuscriptEntry {
                file_name: "paper_3.docx".to_string(),
                file_path: r"C:\docs\paper_3.docx".to_string(),
                word_count: 4200,
                extracted_title: "Updated Title 3".to_string(),
                suggested_keywords: "ai, iot".to_string(),
            },
        );
        assert_eq!(list.len(), MAX_SAVED_MANUSCRIPTS);
        assert_eq!(list[0].file_name, "paper_3.docx");
        assert_eq!(list[0].word_count, 4200);

        save_saved_manuscripts_to(&path, &list).expect("save manuscripts");
        let reloaded = load_saved_manuscripts_from(&path);
        assert_eq!(reloaded, list);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_backup_bundle_serialization_roundtrip() {
        let bundle = JsfsBackupBundle::new(
            serde_json::json!({ "theme": "dark", "polite": true }),
            serde_json::json!([{ "name": "Skripsi AI" }]),
            vec![SavedManuscriptEntry {
                file_name: "skripsi.docx".to_string(),
                file_path: "C:\\skripsi.docx".to_string(),
                word_count: 5500,
                extracted_title: "Analisis Sentimen".to_string(),
                suggested_keywords: "nlp, svm".to_string(),
            }],
            vec!["https://ojs.example.ac.id/journal".to_string()],
            vec![crate::models::UiJournalItem {
                nama: "Jurnal Teknologi Informasi".to_string(),
                url: "https://ojs.example.ac.id/journal".to_string(),
                sinta: "2".to_string(),
                ..Default::default()
            }],
            vec![crate::cache::SubmissionTrack {
                journal_key: "https://ojs.example.ac.id/journal".to_string(),
                status: "Submitted".to_string(),
                submitted_date: "2026-11-17".to_string(),
                notes: "Menunggu reviewer".to_string(),
                updated_at: "2026-11-17T08:00:00Z".to_string(),
            }],
        );

        let json = bundle.to_json_pretty().expect("serialize backup bundle");
        assert!(json.contains("bundle_version"));
        assert!(json.contains("skripsi.docx"));
        assert!(json.contains("Jurnal Teknologi Informasi"));
        assert!(json.contains("Menunggu reviewer"));

        let restored = JsfsBackupBundle::from_json_str(&json).expect("deserialize backup bundle");
        assert_eq!(restored.bundle_version, "1.0");
        assert_eq!(restored.saved_manuscripts.len(), 1);
        assert_eq!(restored.shortlisted_keys.len(), 1);
        assert_eq!(restored.saved_shortlist.len(), 1);
        assert_eq!(restored.saved_shortlist[0].sinta, "2");
        assert_eq!(restored.submission_tracks.len(), 1);
        assert_eq!(restored.submission_tracks[0].status, "Submitted");
    }

    #[test]
    fn test_saved_shortlist_persistence() {
        let dir = std::env::temp_dir().join(format!("jsfs_shortlist_test_{}", std::process::id()));
        let path = dir.join("saved_shortlist.json");
        let _ = std::fs::remove_dir_all(&dir);

        let items = vec![
            crate::models::UiJournalItem {
                nama: "Jurnal Rekayasa Sistem".to_string(),
                penerbit: "Universitas Negeri".to_string(),
                url: "https://jrs.example.ac.id".to_string(),
                sinta: "2".to_string(),
                apc_label: "Rp 0".to_string(),
                ..Default::default()
            },
            crate::models::UiJournalItem {
                nama: "Jurnal Sains Komputer".to_string(),
                penerbit: "Institut Teknologi".to_string(),
                url: "https://jsk.example.ac.id".to_string(),
                sinta: "3".to_string(),
                apc_label: "Rp 500.000".to_string(),
                ..Default::default()
            },
        ];

        save_saved_shortlist_to(&path, &items).expect("save shortlist");
        let loaded = load_saved_shortlist_from(&path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].nama, "Jurnal Rekayasa Sistem");
        assert_eq!(loaded[0].sinta, "2");
        assert_eq!(loaded[1].nama, "Jurnal Sains Komputer");
        assert_eq!(loaded[1].apc_label, "Rp 500.000");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

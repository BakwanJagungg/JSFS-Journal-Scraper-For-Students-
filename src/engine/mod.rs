pub mod ai_ops;
pub mod cache_ops;
pub mod file_ops;
pub mod manuscript;
pub mod search;

use crate::config::MatcherConfig;
use crate::models::{JournalRecord, ManuscriptMeta, ProgressEvent, UiJournalItem};
use crate::{cache, config, scraper};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};

use self::ai_ops::{AiActionResp, AiStatusExtended};
use self::manuscript::ManuscriptSummary;

/// Events emitted asynchronously from background Tokio workers to the native GUI thread.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    LogLine(String),
    Progress(ProgressEvent),
    ResultBatch(Vec<UiJournalItem>),
    AutosaveDone {
        path: String,
        count: usize,
    },
    TargetedExportDone {
        ext: String,
        path: String,
        count: usize,
        scope_label: String,
    },
    SearchComplete {
        results: Vec<UiJournalItem>,
        excel_path: String,
        duration_sec: f64,
        cancelled: bool,
    },
    AiStatusUpdated(AiStatusExtended),
    AiActionDone(AiActionResp),
    AiPullProgress {
        running: bool,
        model_installed: bool,
        task_running: bool,
        task_percent: f64,
        task_message: String,
    },
    ManuscriptLoaded(Result<ManuscriptSummary, String>),
    CacheCleared(Result<(usize, usize), String>),
    SingleUrlAnalyzed(Box<Result<UiJournalItem, String>>),
    FxRateUpdated(Result<i64, String>),
    Toast(String),
}

/// Thread-safe event broadcaster that wakes the `egui` render loop immediately upon delivery.
#[derive(Clone)]
pub struct EventSink {
    tx: mpsc::UnboundedSender<EngineEvent>,
    ctx: Arc<Mutex<Option<egui::Context>>>,
}

impl EventSink {
    pub fn new(tx: mpsc::UnboundedSender<EngineEvent>) -> Self {
        Self {
            tx,
            ctx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn bind_egui_ctx(&self, ctx: egui::Context) {
        if let Ok(mut guard) = self.ctx.lock() {
            *guard = Some(ctx);
        }
    }

    pub fn send(&self, event: EngineEvent) {
        let _ = self.tx.send(event);
        if let Ok(guard) = self.ctx.lock() {
            if let Some(ref ctx) = *guard {
                ctx.request_repaint();
            }
        }
    }
}

/// Shared backend state decoupled completely from any UI framework.
pub struct EngineState {
    pub cancel_flag: Arc<AtomicBool>,
    pub is_running: Arc<AtomicBool>,
    pub current_results: Arc<RwLock<Vec<JournalRecord>>>,
    pub manuscript: Arc<RwLock<Option<ManuscriptMeta>>>,
    pub last_excel_path: Arc<RwLock<Option<String>>>,
    pub cache: Arc<Mutex<Option<cache::JournalCache>>>,
    pub http_client: reqwest::Client,
    pub matcher_config: Arc<RwLock<MatcherConfig>>,
    pub runtime: tokio::runtime::Runtime,
}

impl EngineState {
    pub fn new() -> Self {
        let http_client = scraper::build_http_client();
        let db = cache::JournalCache::open_default().ok();
        let matcher_cfg = config::load_matcher_config();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize Tokio multi-thread runtime");
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
            current_results: Arc::new(RwLock::new(Vec::new())),
            manuscript: Arc::new(RwLock::new(None)),
            last_excel_path: Arc::new(RwLock::new(None)),
            cache: Arc::new(Mutex::new(db)),
            http_client,
            matcher_config: Arc::new(RwLock::new(matcher_cfg)),
            runtime,
        }
    }

    pub fn checkpoint_cache(&self) {
        if let Ok(guard) = self.cache.lock() {
            if let Some(ref db) = *guard {
                db.checkpoint_wal();
            }
        }
    }

    pub fn is_busy(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

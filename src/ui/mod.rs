//! Root native application shell coordinating the navigation sidebar, 5 views, live events, and detail modal.

pub mod detail_modal;
pub mod filters_page;
pub mod guide_page;
pub mod history_page;
pub mod match_engine_page;
pub mod search_page;
pub mod sidebar;
pub mod state;
pub mod theme;

use crate::engine::ai_ops::{refresh_ai_status_async, AiStatusExtended};
use crate::engine::manuscript::{clear_manuscript_async, load_manuscript_async, ManuscriptSummary};
use crate::engine::search::{start_search, stop_search};
use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::models::UiJournalItem;
use crate::ui::detail_modal::render_detail_modal;
use crate::ui::filters_page::render_filters_page;
use crate::ui::guide_page::render_guide_page;
use crate::ui::history_page::render_history_page;
use crate::ui::match_engine_page::render_match_engine_page;
use crate::ui::search_page::{render_search_page, SearchPageAction};
use crate::ui::sidebar::{render_sidebar, NavPage, SidebarProps};
use crate::ui::state::{
    ApcFilter, FilterSettings, MatcherFormState, SearchProgressView, SortColumn,
};
use crate::ui::theme::{apply_theme, load_crisp_fonts, Palette, ThemeMode};
use egui::{Align2, CentralPanel, CornerRadius, Frame, Margin, Panel, RichText, Stroke, Vec2};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub struct JsfsNativeApp {
    engine: Arc<EngineState>,
    sink: EventSink,
    rx: mpsc::UnboundedReceiver<EngineEvent>,

    theme_mode: ThemeMode,
    current_page: NavPage,

    keyword: String,
    target_text: String,
    filters: FilterSettings,
    manuscript: Option<ManuscriptSummary>,

    results: Vec<UiJournalItem>,
    selected_idx: Option<usize>,
    detail_open_idx: Option<usize>,

    table_filter: String,
    apc_filter: ApcFilter,
    sort_col: SortColumn,
    sort_desc: bool,

    progress: SearchProgressView,
    logs: Vec<String>,
    show_logs: bool,
    last_excel_path: Option<String>,

    ai_status: AiStatusExtended,
    matcher_form: MatcherFormState,

    toast: Option<(String, Instant)>,
    fonts_loaded: bool,
}

impl JsfsNativeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        load_crisp_fonts(&cc.egui_ctx);
        let theme_mode = ThemeMode::Dark;
        apply_theme(&cc.egui_ctx, &Palette::from_mode(theme_mode));

        let engine = Arc::new(EngineState::new());
        let (tx, rx) = mpsc::unbounded_channel();
        let sink = EventSink::new(tx);
        sink.bind_egui_ctx(cc.egui_ctx.clone());

        refresh_ai_status_async(engine.clone(), sink.clone());
        let filters = FilterSettings::default();
        let target_text = filters.target_count.to_string();

        Self {
            engine,
            sink,
            rx,
            theme_mode,
            current_page: NavPage::Search,
            keyword: String::new(),
            target_text,
            filters,
            manuscript: None,
            results: Vec::new(),
            selected_idx: None,
            detail_open_idx: None,
            table_filter: String::new(),
            apc_filter: ApcFilter::All,
            sort_col: SortColumn::MatchScore,
            sort_desc: true,
            progress: SearchProgressView::default(),
            logs: Vec::new(),
            show_logs: false,
            last_excel_path: None,
            ai_status: AiStatusExtended {
                active_engine_label: "Match Engine: Built-in Smart".to_string(),
                matcher_mode: "builtin".to_string(),
                ..Default::default()
            },
            matcher_form: MatcherFormState::default(),
            toast: None,
            fonts_loaded: true,
        }
    }

    fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    fn drain_events(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                EngineEvent::LogLine(line) => {
                    self.logs.push(line);
                    if self.logs.len() > 400 {
                        let excess = self.logs.len() - 400;
                        self.logs.drain(..excess);
                    }
                }
                EngineEvent::Progress(p) => {
                    self.progress = SearchProgressView {
                        phase: p.phase,
                        current: p.current,
                        total: p.total,
                        percent: p.percent,
                        current_item: p.current_item,
                        status_text: p.status_text,
                        rate_per_sec: p.rate_per_sec,
                        elapsed_seconds: p.elapsed_seconds,
                        eta_seconds: p.eta_seconds,
                        kpi_total: p.kpi_total,
                        kpi_free: p.kpi_free,
                        kpi_paid: p.kpi_paid,
                        kpi_fast: p.kpi_fast,
                    };
                }
                EngineEvent::ResultBatch(batch) => {
                    self.results.extend(batch);
                }
                EngineEvent::AutosaveDone { path, count } => {
                    self.last_excel_path = Some(path.clone());
                    self.logs.push(format!("Autosave sementara ({count} jurnal): {path}"));
                }
                EngineEvent::SearchComplete {
                    results,
                    excel_path,
                    duration_sec,
                    cancelled,
                } => {
                    self.results = results;
                    if !excel_path.trim().is_empty() {
                        self.last_excel_path = Some(excel_path);
                    }
                    self.progress.elapsed_seconds = duration_sec;
                    self.progress.eta_seconds = None;
                    self.progress.status_text = if cancelled {
                        format!("Dihentikan ({} jurnal parsial)", self.results.len())
                    } else {
                        format!("Selesai ({} jurnal ditemukan)", self.results.len())
                    };
                    self.progress.current_item = self.progress.status_text.clone();
                    let msg = if cancelled {
                        format!("Pencarian dihentikan ({} jurnal).", self.results.len())
                    } else {
                        format!(
                            "Selesai dalam {duration_sec:.1} dtk — {} jurnal valid!",
                            self.results.len()
                        )
                    };
                    self.set_toast(msg);
                    refresh_ai_status_async(self.engine.clone(), self.sink.clone());
                }
                EngineEvent::AiStatusUpdated(st) => {
                    self.matcher_form.sync_from_status(&st);
                    self.ai_status = st;
                }
                EngineEvent::AiActionDone(resp) => {
                    self.matcher_form.cloud_test_result = Some((resp.ok, resp.message.clone()));
                    self.set_toast(resp.message);
                }
                EngineEvent::AiPullProgress {
                    running,
                    model_installed,
                    task_running,
                    task_percent,
                    task_message,
                } => {
                    self.ai_status.running = running;
                    self.ai_status.model_installed = model_installed;
                    self.matcher_form.pull_running = task_running;
                    self.matcher_form.pull_percent = task_percent;
                    self.matcher_form.pull_message = task_message;
                }
                EngineEvent::ManuscriptLoaded(Ok(summary)) => {
                    if self.keyword.trim().is_empty() && !summary.suggested_keyword.is_empty() {
                        self.keyword = summary.suggested_keyword.clone();
                    }
                    self.logs.push(format!("Naskah dimuat: {}", summary.name));
                    self.set_toast(format!("Naskah '{}' siap dianalisis!", summary.name));
                    self.manuscript = Some(summary);
                }
                EngineEvent::ManuscriptLoaded(Err(err)) => {
                    self.set_toast(format!("Gagal membaca naskah: {err}"));
                }
                EngineEvent::CacheCleared(Ok((cleared, remaining))) => {
                    self.ai_status.cache_count = remaining;
                    self.logs.push(format!("Cache SQLite dibersihkan: {cleared} entri dihapus."));
                }
                EngineEvent::CacheCleared(Err(err)) => {
                    self.set_toast(format!("Gagal membersihkan cache: {err}"));
                }
                EngineEvent::Toast(msg) => {
                    self.set_toast(msg);
                }
            }
        }

        if let Some((_, ts)) = &self.toast {
            if ts.elapsed() > Duration::from_secs(4) {
                self.toast = None;
            }
        }
    }

    fn trigger_search(&mut self) {
        if self.keyword.trim().is_empty() {
            self.set_toast("Ketik topik atau kata kunci penelitian terlebih dahulu.");
            return;
        }
        if !self.filters.garuda && !self.filters.sinta_src {
            self.set_toast("Aktifkan minimal satu portal (Garuda / SINTA) di Filter Lanjutan.");
            return;
        }
        if let Ok(n) = self.target_text.trim().parse::<usize>() {
            self.filters.clamp_target(n);
            self.target_text = self.filters.target_count.to_string();
        }
        let params = self
            .filters
            .to_search_params(&self.keyword, &self.matcher_form.ollama_model);

        self.results.clear();
        self.selected_idx = None;
        self.detail_open_idx = None;
        self.progress = SearchProgressView {
            phase: "fase1_portal".to_string(),
            status_text: "Menghubungi portal SINTA & Garuda...".to_string(),
            current_item: format!("Mencari '{}'", self.keyword.trim()),
            ..Default::default()
        };

        if let Err(e) = start_search(self.engine.clone(), self.sink.clone(), params) {
            self.set_toast(e.to_string());
        }
    }
}

impl eframe::App for JsfsNativeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.fonts_loaded {
            load_crisp_fonts(ui.ctx());
            self.fonts_loaded = true;
        }
        self.drain_events();

        let p = Palette::from_mode(self.theme_mode);
        let engine_ready = match self.ai_status.matcher_mode.as_str() {
            "cloud" => self.ai_status.cloud_has_key,
            "ollama" => self.ai_status.running,
            _ => true,
        };
        let is_searching = self.engine.is_busy();
        if is_searching || self.toast.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }

        // Left Navigation Sidebar
        Panel::left("jsfs_sidebar")
            .resizable(false)
            .exact_size(224.0)
            .frame(
                Frame::new()
                    .fill(p.sidebar_bg)
                    .stroke(Stroke::new(1.0, p.line))
                    .inner_margin(Margin::symmetric(10, 8)),
            )
            .show(ui, |ui| {
                let side_action = render_sidebar(
                    ui,
                    &p,
                    SidebarProps {
                        current: self.current_page,
                        engine_label: &self.ai_status.active_engine_label,
                        engine_ready,
                        is_searching,
                        result_count: self.results.len(),
                        active_filter_count: self.filters.active_filter_count(),
                        theme_mode: self.theme_mode,
                    },
                );
                if let Some(page) = side_action.selected {
                    self.current_page = page;
                }
                if side_action.toggle_theme {
                    self.theme_mode = self.theme_mode.toggled();
                    apply_theme(ui.ctx(), &Palette::from_mode(self.theme_mode));
                }
            });

        // Main Content Area
        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(p.content_bg)
                    .inner_margin(Margin::symmetric(22, 14)),
            )
            .show(ui, |ui| match self.current_page {
                NavPage::Search => {
                    let act = render_search_page(
                        ui,
                        &p,
                        &mut self.keyword,
                        &mut self.target_text,
                        &mut self.filters,
                        self.manuscript.as_ref(),
                        is_searching,
                        &self.progress,
                        &self.results,
                        &mut self.table_filter,
                        &mut self.apc_filter,
                        &mut self.sort_col,
                        &mut self.sort_desc,
                        &mut self.show_logs,
                        &self.logs,
                        &self.sink,
                    );
                    match act {
                        SearchPageAction::StartSearch => self.trigger_search(),
                        SearchPageAction::StopSearch => {
                            stop_search(&self.engine);
                            self.set_toast("Menghentikan pencarian...");
                        }
                        SearchPageAction::PickManuscript(path) => {
                            load_manuscript_async(self.engine.clone(), self.sink.clone(), path);
                        }
                        SearchPageAction::ClearManuscript => {
                            self.manuscript = None;
                            clear_manuscript_async(self.engine.clone(), self.sink.clone());
                        }
                        SearchPageAction::OpenDetail(idx) => {
                            self.selected_idx = Some(idx);
                            self.detail_open_idx = Some(idx);
                        }
                        SearchPageAction::NavigateFilters => {
                            self.current_page = NavPage::Filters;
                        }
                        SearchPageAction::None => {}
                    }
                }
                NavPage::Filters => {
                    render_filters_page(
                        ui,
                        &p,
                        &mut self.filters,
                        &mut self.target_text,
                        &self.sink,
                    );
                }
                NavPage::MatchEngine => {
                    render_match_engine_page(
                        ui,
                        &p,
                        &mut self.matcher_form,
                        &self.ai_status,
                        &self.engine,
                        &self.sink,
                    );
                }
                NavPage::History => {
                    render_history_page(
                        ui,
                        &p,
                        &self.results,
                        self.selected_idx,
                        &self.keyword,
                        self.last_excel_path.as_deref(),
                        self.ai_status.cache_count,
                        &mut self.logs,
                        &self.engine,
                        &self.sink,
                    );
                }
                NavPage::Guide => {
                    render_guide_page(ui, &p, &self.sink);
                }
            });

        // Journal Detail Modal
        if let Some(idx) = self.detail_open_idx {
            let item = self.results.get(idx);
            if render_detail_modal(ui.ctx(), &p, item, &self.keyword, &self.sink) {
                self.detail_open_idx = None;
            }
        }

        // Floating Toast Notification
        if let Some((msg, _)) = &self.toast {
            egui::Area::new(egui::Id::unique("jsfs_toast"))
                .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-20.0, -20.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    Frame::new()
                        .fill(p.card_bg)
                        .stroke(Stroke::new(1.2, p.accent))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(14, 9))
                        .show(ui, |ui| {
                            ui.label(RichText::new(msg).size(12.5).color(p.text));
                        });
                });
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.engine.cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        self.engine.checkpoint_cache();
    }
}

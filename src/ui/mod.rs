//! Root native application shell coordinating the navigation sidebar, 5 views, live events, and detail modal.

pub mod compare_modal;
pub mod detail_modal;
pub mod filters_page;
pub mod guide_page;
pub mod history_page;
pub mod i18n;
pub mod match_engine_page;
pub mod search_page;
pub mod settings_page;
pub mod sidebar;
pub mod state;
pub mod theme;

use crate::engine::ai_ops::{refresh_ai_status_async, AiStatusExtended};
use crate::engine::manuscript::{clear_manuscript_async, load_manuscript_async, ManuscriptSummary};
use crate::engine::search::{start_search, stop_search};
use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::models::{reformat_ui_results_currency, DisplayCurrency, UiJournalItem};
use crate::parser::doc_matcher::apply_cited_counts;
use crate::ui::compare_modal::{render_compare_modal, CompareModalAction};
use crate::ui::detail_modal::{render_detail_modal, DetailModalAction};
use crate::ui::filters_page::render_filters_page;
use crate::ui::guide_page::render_guide_page;
use crate::ui::history_page::{render_history_page, HistoryPageAction};
use crate::ui::i18n::{load_app_settings, save_app_settings, t, AppSettings};
use crate::ui::match_engine_page::render_match_engine_page;
use crate::ui::search_page::{render_search_page, SearchPageAction};
use crate::ui::settings_page::render_settings_page;
use crate::ui::sidebar::{render_sidebar, NavPage, SidebarProps};
use crate::ui::state::{
    load_saved_profiles, load_search_history, record_search_history, ApcFilter, AppState,
    FilterSettings, MatcherFormState, SearchHistoryEntry, SearchProgressView, SortColumn,
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
    compare_open: bool,

    table_filter: String,
    apc_filter: ApcFilter,
    app_state: AppState,
    sort_col: SortColumn,
    sort_desc: bool,

    progress: SearchProgressView,
    logs: Vec<String>,
    show_history_logs: bool,
    last_excel_path: Option<String>,
    search_history: Vec<SearchHistoryEntry>,

    app_settings: AppSettings,
    usd_rate_text: String,
    last_currency_sig: (DisplayCurrency, i64),

    ai_status: AiStatusExtended,
    matcher_form: MatcherFormState,

    toast: Option<(String, Instant)>,
    fonts_loaded: bool,
}

impl JsfsNativeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        load_crisp_fonts(&cc.egui_ctx);
        let app_settings = load_app_settings();
        crate::scraper::set_polite_bot_mode(app_settings.polite_bot_mode);
        let theme_mode = if app_settings.dark_theme {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        apply_theme(&cc.egui_ctx, &Palette::from_mode(theme_mode));

        let engine = Arc::new(EngineState::new());
        let (tx, rx) = mpsc::unbounded_channel();
        let sink = EventSink::new(tx);
        sink.bind_egui_ctx(cc.egui_ctx.clone());

        refresh_ai_status_async(engine.clone(), sink.clone());
        let mut filters = FilterSettings::default();
        filters.apply_app_settings(&app_settings);
        let target_text = filters.target_count.to_string();
        let usd_rate_text = app_settings.usd_to_idr.to_string();
        let submission_tracks = engine
            .cache
            .lock()
            .ok()
            .and_then(|g| {
                g.as_ref()
                    .and_then(|db| db.load_all_submission_tracks().ok())
            })
            .unwrap_or_default();
        let app_state = AppState {
            saved_profiles: load_saved_profiles(),
            submission_tracks,
            ..Default::default()
        };

        let last_currency_sig = (app_settings.display_currency, app_settings.usd_to_idr);
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
            compare_open: false,
            table_filter: String::new(),
            apc_filter: ApcFilter::All,
            app_state,
            sort_col: SortColumn::MatchScore,
            sort_desc: true,
            progress: SearchProgressView::default(),
            logs: Vec::new(),
            show_history_logs: false,
            last_excel_path: None,
            search_history: load_search_history(),
            app_settings,
            usd_rate_text,
            last_currency_sig,
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
                EngineEvent::ResultBatch(mut batch) => {
                    reformat_ui_results_currency(
                        &mut batch,
                        self.app_settings.display_currency,
                        self.app_settings.usd_to_idr,
                    );
                    if let Some(ref ms) = self.manuscript {
                        apply_cited_counts(&mut batch, &ms.cited_journals);
                    }
                    self.results.extend(batch);
                }
                EngineEvent::AutosaveDone { path, count } => {
                    self.last_excel_path = Some(path.clone());
                    self.logs
                        .push(format!("Autosave sementara ({count} jurnal): {path}"));
                }
                EngineEvent::TargetedExportDone {
                    ext,
                    path,
                    count,
                    scope_label,
                } => {
                    if ext == "xlsx" {
                        self.last_excel_path = Some(path.clone());
                    }
                    let fname = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&path);
                    self.logs.push(format!(
                        "Ekspor .{ext} ({count} jurnal - {scope_label}): {path}"
                    ));
                    self.set_toast(format!(
                        "Ekspor .{ext} ({count} jurnal - {scope_label}) tersimpan: {fname}"
                    ));
                }
                EngineEvent::SearchComplete {
                    mut results,
                    excel_path,
                    duration_sec,
                    cancelled,
                } => {
                    reformat_ui_results_currency(
                        &mut results,
                        self.app_settings.display_currency,
                        self.app_settings.usd_to_idr,
                    );
                    if let Some(ref ms) = self.manuscript {
                        apply_cited_counts(&mut results, &ms.cited_journals);
                    }
                    self.results = results;
                    if !excel_path.trim().is_empty() {
                        self.last_excel_path = Some(excel_path);
                    }
                    if !self.results.is_empty() {
                        record_search_history(
                            &mut self.search_history,
                            &self.keyword,
                            &self.filters,
                            &self.results,
                        );
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
                            "Selesai dalam {duration_sec:.1} dtk - {} jurnal valid!",
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
                    let auto_filled = if self.keyword.trim().is_empty()
                        && !summary.suggested_keyword.trim().is_empty()
                    {
                        self.keyword = summary.suggested_keyword.clone();
                        true
                    } else {
                        false
                    };
                    self.logs.push(format!(
                        "Naskah dimuat: {} (Saran keyword: {})",
                        summary.name, summary.suggested_keyword
                    ));
                    let msg = if auto_filled {
                        format!(
                            "Naskah '{}' dimuat & kata kunci diisi: {}",
                            summary.name, summary.suggested_keyword
                        )
                    } else {
                        format!("Naskah '{}' siap dianalisis!", summary.name)
                    };
                    self.set_toast(msg);
                    apply_cited_counts(&mut self.results, &summary.cited_journals);
                    self.manuscript = Some(summary);
                }
                EngineEvent::ManuscriptLoaded(Err(err)) => {
                    self.set_toast(format!("Gagal membaca naskah: {err}"));
                }
                EngineEvent::CacheCleared(Ok((cleared, remaining))) => {
                    self.ai_status.cache_count = remaining;
                    self.logs.push(format!(
                        "Cache SQLite dibersihkan: {cleared} entri dihapus."
                    ));
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
        let txt = t(self.app_settings.language);
        if self.keyword.trim().is_empty() {
            self.set_toast(txt.toast_empty_kw);
            return;
        }
        if !self.filters.garuda
            && !self.filters.sinta_src
            && !self.filters.openalex
            && !self.filters.doaj
        {
            self.set_toast(txt.toast_no_portal);
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
                        lang: self.app_settings.language,
                    },
                );
                if let Some(page) = side_action.selected {
                    self.current_page = page;
                }
                if side_action.toggle_theme {
                    self.theme_mode = self.theme_mode.toggled();
                    self.app_settings.dark_theme = self.theme_mode == ThemeMode::Dark;
                    let _ = save_app_settings(&self.app_settings);
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
                        self.app_settings.language,
                        &mut self.keyword,
                        &mut self.target_text,
                        &mut self.filters,
                        &mut self.app_state,
                        self.manuscript.as_ref(),
                        is_searching,
                        &self.progress,
                        &self.results,
                        &mut self.table_filter,
                        &mut self.apc_filter,
                        &mut self.sort_col,
                        &mut self.sort_desc,
                        &self.sink,
                    );
                    match act {
                        SearchPageAction::StartSearch => self.trigger_search(),
                        SearchPageAction::StopSearch => {
                            stop_search(&self.engine);
                            self.set_toast(t(self.app_settings.language).toast_stopping);
                        }
                        SearchPageAction::PickManuscript(path) => {
                            load_manuscript_async(self.engine.clone(), self.sink.clone(), path);
                        }
                        SearchPageAction::ClearManuscript => {
                            self.manuscript = None;
                            apply_cited_counts(
                                &mut self.results,
                                &std::collections::HashMap::new(),
                            );
                            clear_manuscript_async(self.engine.clone(), self.sink.clone());
                        }
                        SearchPageAction::OpenDetail(idx) => {
                            self.selected_idx = Some(idx);
                            self.detail_open_idx = Some(idx);
                        }
                        SearchPageAction::OpenCompare => {
                            self.compare_open = true;
                        }
                        SearchPageAction::CopyWa(indices) => {
                            let refs: Vec<&UiJournalItem> = indices
                                .iter()
                                .filter_map(|&idx| self.results.get(idx))
                                .collect();
                            if !refs.is_empty() {
                                let text = crate::engine::file_ops::format_wa_share_from_ui(&refs);
                                ui.ctx().copy_text(text);
                                self.set_toast(format!(
                                    "Daftar WA untuk {} jurnal disalin ke clipboard.",
                                    refs.len()
                                ));
                            }
                        }
                        SearchPageAction::OpenExcel => {
                            crate::engine::file_ops::open_excel_async(
                                self.engine.clone(),
                                self.sink.clone(),
                                self.last_excel_path.clone(),
                            );
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
                        self.app_settings.language,
                        &mut self.filters,
                        &mut self.target_text,
                        &self.sink,
                    );
                }
                NavPage::MatchEngine => {
                    render_match_engine_page(
                        ui,
                        &p,
                        self.app_settings.language,
                        &mut self.matcher_form,
                        &self.ai_status,
                        &self.engine,
                        &self.sink,
                    );
                }
                NavPage::History => {
                    match render_history_page(
                        ui,
                        &p,
                        self.app_settings.language,
                        &self.results,
                        self.selected_idx,
                        &self.keyword,
                        &mut self.app_state,
                        &self.table_filter,
                        self.apc_filter,
                        self.sort_col,
                        self.sort_desc,
                        &self.search_history,
                        self.last_excel_path.as_deref(),
                        self.ai_status.cache_count,
                        &mut self.logs,
                        &mut self.show_history_logs,
                        &self.engine,
                        &self.sink,
                    ) {
                        HistoryPageAction::LoadHistoryResults(idx) => {
                            if let Some(entry) = self.search_history.get(idx).cloned() {
                                self.keyword = entry.keyword.clone();
                                self.results = entry.records;
                                reformat_ui_results_currency(
                                    &mut self.results,
                                    self.app_settings.display_currency,
                                    self.app_settings.usd_to_idr,
                                );
                                if let Some(ref ms) = self.manuscript {
                                    apply_cited_counts(&mut self.results, &ms.cited_journals);
                                }
                                self.selected_idx = None;
                                self.detail_open_idx = None;
                                let kpi_total = self.results.len();
                                let kpi_free = self
                                    .results
                                    .iter()
                                    .filter(|r| r.apc_category == "free")
                                    .count();
                                let kpi_paid = self
                                    .results
                                    .iter()
                                    .filter(|r| r.apc_category == "paid")
                                    .count();
                                self.progress = SearchProgressView {
                                    phase: "fase4_export".to_string(),
                                    status_text: format!("Riwayat dimuat ({kpi_total} jurnal)"),
                                    current_item: format!("Sesi {}", entry.timestamp),
                                    kpi_total,
                                    kpi_free,
                                    kpi_paid,
                                    kpi_fast: 0,
                                    ..Default::default()
                                };
                                self.current_page = NavPage::Search;
                                self.set_toast(format!(
                                    "Memuat {kpi_total} jurnal untuk '{}'.",
                                    entry.keyword
                                ));
                            }
                        }
                        HistoryPageAction::ApplyHistoryFilter(idx) => {
                            if let Some(entry) = self.search_history.get(idx).cloned() {
                                self.filters = entry.filter_snapshot;
                                self.target_text = self.filters.target_count.to_string();
                                if self.keyword.trim().is_empty()
                                    && !entry.keyword.trim().is_empty()
                                {
                                    self.keyword = entry.keyword;
                                }
                                self.current_page = NavPage::Filters;
                                self.set_toast(format!(
                                    "Filter riwayat diterapkan: {}",
                                    self.filters.summary_text()
                                ));
                            }
                        }
                        HistoryPageAction::None => {}
                    }
                }
                NavPage::Settings => {
                    render_settings_page(
                        ui,
                        &p,
                        &mut self.app_settings,
                        &mut self.theme_mode,
                        &mut self.filters,
                        &mut self.usd_rate_text,
                        &self.engine,
                        &self.sink,
                    );
                }
                NavPage::Guide => {
                    render_guide_page(ui, &p, self.app_settings.language, &self.sink);
                }
            });

        let cur_sig = (
            self.app_settings.display_currency,
            self.app_settings.usd_to_idr,
        );
        if cur_sig != self.last_currency_sig {
            reformat_ui_results_currency(&mut self.results, cur_sig.0, cur_sig.1);
            self.last_currency_sig = cur_sig;
        }

        // Journal Detail Modal
        if let Some(idx) = self.detail_open_idx {
            let item = self.results.get(idx).cloned();
            match render_detail_modal(
                ui.ctx(),
                &p,
                idx,
                item.as_ref(),
                &self.keyword,
                self.manuscript.as_ref(),
                &self.app_settings,
                &mut self.app_state,
                &self.sink,
            ) {
                DetailModalAction::Close => {
                    self.detail_open_idx = None;
                }
                DetailModalAction::ExportBundle(target_idx) => {
                    if let Some(j_item) = self.results.get(target_idx).cloned() {
                        let lang_code =
                            if self.app_settings.language == crate::ui::i18n::AppLanguage::En {
                                "en"
                            } else {
                                "id"
                            };
                        crate::engine::file_ops::export_submission_bundle_async(
                            self.engine.clone(),
                            self.sink.clone(),
                            j_item,
                            self.keyword.clone(),
                            self.manuscript.clone(),
                            self.app_settings.clone(),
                            lang_code,
                        );
                    }
                }
                DetailModalAction::FindSimilar(target_idx) => {
                    let seed_name = self
                        .results
                        .get(target_idx)
                        .map(|r| r.nama.clone())
                        .unwrap_or_default();
                    let ranked = crate::parser::doc_matcher::rank_similar_journals(
                        target_idx,
                        &self.results,
                    );
                    for (r_idx, sim_sc) in ranked {
                        if let Some(row) = self.results.get_mut(r_idx) {
                            row.match_score = sim_sc;
                            row.kesesuaian_naskah = if sim_sc >= 75.0 {
                                format!("Mirip '{seed_name}'")
                            } else {
                                "Topik Serupa".to_string()
                            };
                        }
                    }
                    self.sort_col = SortColumn::MatchScore;
                    self.sort_desc = true;
                    self.detail_open_idx = None;
                    self.set_toast(format!(
                        "Tabel diurutkan berdasarkan kemiripan topik dengan '{seed_name}'."
                    ));
                }
                DetailModalAction::SaveTrack(track) => {
                    if let Ok(guard) = self.engine.cache.lock() {
                        if let Some(ref db) = *guard {
                            let _ = db.upsert_submission_track(&track);
                        }
                    }
                    self.app_state
                        .submission_tracks
                        .insert(track.journal_key.clone(), track.clone());
                    self.set_toast(format!(
                        "Status pengiriman '{}' disimpan ke SQLite.",
                        track.status
                    ));
                }
                DetailModalAction::None => {}
            }
        }

        // Side-by-Side Compare Modal (2..=4 shortlisted journals)
        if self.compare_open {
            let compared: Vec<(usize, UiJournalItem)> = self
                .results
                .iter()
                .enumerate()
                .filter(|(_, r)| self.app_state.is_bookmarked(r))
                .take(4)
                .map(|(i, r)| (i, r.clone()))
                .collect();
            match render_compare_modal(
                ui.ctx(),
                &p,
                &compared,
                self.manuscript.as_ref(),
                self.app_settings.language,
            ) {
                CompareModalAction::Close => {
                    self.compare_open = false;
                }
                CompareModalAction::OpenDetail(idx) => {
                    self.compare_open = false;
                    self.selected_idx = Some(idx);
                    self.detail_open_idx = Some(idx);
                }
                CompareModalAction::None => {}
            }
        }

        // Floating Toast Notification (fixed horizontal banner width so it never wraps vertically)
        if let Some((msg, _)) = &self.toast {
            egui::Area::new(egui::Id::unique("jsfs_toast"))
                .anchor(Align2::RIGHT_TOP, Vec2::new(-20.0, 16.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    Frame::new()
                        .fill(p.card_bg)
                        .stroke(Stroke::new(1.2, p.accent))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(14, 9))
                        .show(ui, |ui| {
                            ui.set_min_width(280.0);
                            ui.set_max_width(420.0);
                            ui.add(
                                egui::Label::new(RichText::new(msg).size(12.5).color(p.text))
                                    .wrap(),
                            );
                        });
                });
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.engine
            .cancel_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.engine.checkpoint_cache();
    }
}

//! 4-Phase Async Search Pipeline (`start_search` & `stop_search`) implementing Filter-Before-Deep,
//! Semaphore(32) concurrent OJS crawl, incremental auto-save every 50 journals, and instant cancel.

use crate::ai::ollama;
use crate::config::{AUTOSAVE_INTERVAL_ITEMS, MAX_WORKERS};
use crate::error::JsfsError;
use crate::exporter;
use crate::models::{JournalRecord, ProgressEvent, SearchParams, UiJournalItem};
use crate::parser::{doc_matcher, month};
use crate::scraper::{dedup, garuda, ojs_deep, sinta, DomainBreaker};
use crate::AppState;
use chrono::{Datelike, Local};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

#[derive(Serialize)]
pub struct StartSearchResp {
    pub ok: bool,
    pub run_id: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct OkResp {
    pub ok: bool,
}

#[derive(Clone, Serialize)]
struct ScrapeCompleteEvent {
    pub results: Vec<UiJournalItem>,
    pub excel_path: String,
    pub duration_sec: f64,
    pub total_count: usize,
    pub cancelled: bool,
}

#[derive(Clone, Serialize)]
struct AutosaveEvent {
    pub path: String,
    pub count: usize,
}

fn emit_log(app: &AppHandle, msg: impl Into<String>) {
    let _ = app.emit("log-line", msg.into());
}

fn compute_kpi(items: &[JournalRecord]) -> (usize, usize, usize, usize) {
    let cur_m = Local::now().month() as u8;
    let near = [cur_m, (cur_m % 12) + 1, ((cur_m + 1) % 12) + 1];
    let total = items.len();
    let free = items.iter().filter(|r| r.apc_category() == "free").count();
    let paid = items.iter().filter(|r| r.apc_category() == "paid").count();
    let fast = items
        .iter()
        .filter(|r| r.publish_month_nums.iter().any(|m| near.contains(m)))
        .count();
    (total, free, paid, fast)
}

#[allow(clippy::too_many_arguments)]
fn emit_progress(
    app: &AppHandle,
    phase: &str,
    current: usize,
    total: usize,
    current_item: &str,
    status_text: &str,
    started_at: Instant,
    kpi: (usize, usize, usize, usize),
) {
    let elapsed = started_at.elapsed().as_secs_f64();
    let percent = if total > 0 {
        ((current as f64 / total as f64) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let rate = if elapsed > 0.2 && current > 0 {
        current as f64 / elapsed
    } else {
        0.0
    };
    let eta_seconds = if total > current && rate > 0.05 {
        Some(((total - current) as f64 / rate).ceil() as u64)
    } else {
        None
    };

    let _ = app.emit(
        "scrape-progress",
        ProgressEvent {
            phase: phase.to_string(),
            current,
            total,
            percent,
            current_item: current_item.to_string(),
            eta_seconds,
            status_text: status_text.to_string(),
            rate_per_sec: rate,
            elapsed_seconds: elapsed,
            kpi_total: kpi.0,
            kpi_free: kpi.1,
            kpi_paid: kpi.2,
            kpi_fast: kpi.3,
        },
    );
}

fn filter_sinta_tier(rec: &JournalRecord, allowed_ranks: &[u8]) -> bool {
    if allowed_ranks.is_empty() || allowed_ranks.len() >= 6 {
        return true;
    }
    match rec.sinta_rank_num() {
        Some(r) => allowed_ranks.contains(&r),
        // Allow Garuda records without parsed SINTA badge through candidate pool so cross-ref/user isn't starved
        None => rec.source_portal.contains("Garuda"),
    }
}

fn filter_apc_tier(rec: &JournalRecord, params: &SearchParams) -> bool {
    match rec.apc_category() {
        "free" => true,
        "unknown" => !params.only_free && params.include_na,
        "paid" => {
            if params.only_free {
                return false;
            }
            let idr = rec.publication_fee_idr.unwrap_or(0);
            idr >= params.min_rp && idr <= params.max_rp
        }
        _ => true,
    }
}

#[tauri::command]
pub async fn start_search(
    app: AppHandle,
    state: State<'_, AppState>,
    params: SearchParams,
) -> Result<StartSearchResp, JsfsError> {
    if state.is_running.load(Ordering::SeqCst) {
        return Ok(StartSearchResp {
            ok: false,
            run_id: String::new(),
            message: "Pencarian sedang berjalan.".to_string(),
        });
    }

    let keywords = params.normalized_keywords();
    if keywords.is_empty() {
        return Err(JsfsError::InvalidInput(
            "Kata kunci pencarian wajib diisi.".to_string(),
        ));
    }

    let run_id = format!("run_{}", chrono::Utc::now().timestamp_millis());
    state.cancel_flag.store(false, Ordering::SeqCst);
    state.is_running.store(true, Ordering::SeqCst);

    let client = state.http_client.clone();
    let cache = state.cache.clone();
    let cancel_flag = state.cancel_flag.clone();
    let is_running = state.is_running.clone();
    let current_results = state.current_results.clone();
    let last_excel_path = state.last_excel_path.clone();
    let manuscript_guard = state.manuscript.clone();

    tauri::async_runtime::spawn(async move {
        let started_at = Instant::now();
        let target = params.effective_target();
        let sinta_ranks = params.normalized_sinta_ranks();
        let selected_months = params.normalized_months();
        let breaker = Arc::new(DomainBreaker::new());
        let kw_label = keywords.join(", ");

        {
            let mut res_w = current_results.write().await;
            res_w.clear();
            let mut ex_w = last_excel_path.write().await;
            *ex_w = None;
        }

        emit_log(
            &app,
            format!("Fase 1: Menjelajahi portal untuk '{kw_label}' (Target: {target} jurnal)..."),
        );
        emit_progress(
            &app,
            "fase1_portal",
            0,
            0,
            &format!("Menghubungi SINTA & Garuda untuk '{kw_label}'..."),
            "Menjelajahi halaman portal SINTA & Garuda...",
            started_at,
            (0, 0, 0, 0),
        );

        // ── PHASE 1: Portal Scraping (Garuda + SINTA concurrent per keyword) ────────
        let pool_target = (target * 2).clamp(40, 1500);
        let mut raw_candidates: Vec<JournalRecord> = Vec::new();
        let mut portal_tasks = JoinSet::new();

        for kw in &keywords {
            if params.garuda {
                let c = client.clone();
                let br = breaker.clone();
                let kw_c = kw.clone();
                let sr = sinta_ranks.clone();
                let cf = cancel_flag.clone();
                let app_c = app.clone();
                portal_tasks.spawn(async move {
                    garuda::scrape_garuda(&c, br, &kw_c, &sr, pool_target, cf, move |d, t, msg| {
                        emit_progress(
                            &app_c,
                            "fase1_portal",
                            d,
                            t,
                            msg,
                            "Menjelajahi Portal Garuda...",
                            started_at,
                            (0, 0, 0, 0),
                        );
                    })
                    .await
                });
            }
            if params.sinta_src {
                let c = client.clone();
                let br = breaker.clone();
                let kw_c = kw.clone();
                let sr = sinta_ranks.clone();
                let cf = cancel_flag.clone();
                let app_c = app.clone();
                portal_tasks.spawn(async move {
                    let app_cb = app_c.clone();
                    let (rows, hit_cf) =
                        sinta::scrape_sinta(&c, br, &kw_c, &sr, pool_target, cf, move |d, t, msg| {
                            emit_progress(
                                &app_cb,
                                "fase1_portal",
                                d,
                                t,
                                msg,
                                "Menjelajahi Portal SINTA...",
                                started_at,
                                (0, 0, 0, 0),
                            );
                        })
                        .await;
                    if hit_cf {
                        emit_log(
                            &app_c,
                            "⚠️ SINTA membatasi akses HTTP paralel (Cloudflare); hasil Garuda tetap diproses.",
                        );
                    }
                    rows
                });
            }
        }

        while let Some(joined) = portal_tasks.join_next().await {
            if let Ok(rows) = joined {
                raw_candidates.extend(rows);
            }
        }

        if cancel_flag.load(Ordering::Relaxed) {
            emit_log(&app, "Pencarian dihentikan oleh pengguna.");
            let _ = app.emit(
                "scrape-complete",
                ScrapeCompleteEvent {
                    results: vec![],
                    excel_path: String::new(),
                    duration_sec: started_at.elapsed().as_secs_f64(),
                    total_count: 0,
                    cancelled: true,
                },
            );
            is_running.store(false, Ordering::SeqCst);
            return;
        }

        // Deduplicate cross-source and apply Filter-Before-Deep on SINTA tier
        let deduped = dedup::deduplicate_and_merge(raw_candidates);
        let mut deep_pool: Vec<JournalRecord> = deduped
            .into_iter()
            .filter(|r| filter_sinta_tier(r, &sinta_ranks))
            .collect();
        let deep_cap = (target * 2).clamp(40, 1200);
        deep_pool.truncate(deep_cap);

        emit_log(
            &app,
            format!(
                "Fase 2: Deep Crawl OJS ({} jurnal kandidat setelah deduplikasi & pre-filter SINTA)...",
                deep_pool.len()
            ),
        );

        // ── PHASE 2: Concurrent OJS Deep Crawl (`Semaphore(32)`) + Auto-Save ───────
        let total_deep = deep_pool.len();
        let sem = Arc::new(Semaphore::new(MAX_WORKERS));
        let mut deep_set = JoinSet::new();
        for (idx, rec) in deep_pool.into_iter().enumerate() {
            let c = client.clone();
            let br = breaker.clone();
            let db = cache.clone();
            let sem_c = sem.clone();
            let cf = cancel_flag.clone();
            deep_set.spawn(async move {
                let Ok(_permit) = sem_c.acquire().await else {
                    return (idx, rec);
                };
                if cf.load(Ordering::Relaxed) {
                    return (idx, rec);
                }
                let enriched = ojs_deep::deep_enrich_record(&c, br, db, rec, cf).await;
                (idx, enriched)
            });
        }

        let mut enriched_list: Vec<JournalRecord> = Vec::with_capacity(total_deep);
        let mut pending_batch: Vec<UiJournalItem> = Vec::new();
        while let Some(joined) = deep_set.join_next().await {
            if let Ok((_idx, mut rec)) = joined {
                // Preliminary match score for live UI streaming
                if params.match_enabled {
                    let ms_ref = manuscript_guard.read().await;
                    let (sc, lbl, rsn) =
                        doc_matcher::compute_match_score(&rec, &keywords, ms_ref.as_ref());
                    rec.match_score = sc;
                    rec.match_label = lbl;
                    rec.match_reasons = rsn;
                }
                let done_now = enriched_list.len() + 1;
                pending_batch.push(UiJournalItem::from(&rec));
                let title_snap = rec.title.clone();
                enriched_list.push(rec);

                let kpi = compute_kpi(&enriched_list);
                emit_progress(
                    &app,
                    "fase2_deep",
                    done_now,
                    total_deep,
                    &title_snap,
                    &format!("Memeriksa biaya & jadwal ({done_now}/{total_deep} jurnal)..."),
                    started_at,
                    kpi,
                );

                if pending_batch.len() >= 10 || done_now == total_deep {
                    let _ = app.emit("result-batch", std::mem::take(&mut pending_batch));
                }

                // Incremental auto-save every 50 enriched records
                if done_now.is_multiple_of(AUTOSAVE_INTERVAL_ITEMS) && !enriched_list.is_empty() {
                    let snap = enriched_list.clone();
                    let kw_s = kw_label.clone();
                    let fmts = params.formats.clone();
                    let app_s = app.clone();
                    let ex_s = last_excel_path.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Ok(Ok(bundle)) = tokio::task::spawn_blocking(move || {
                            exporter::write_all_exports_sync(&snap, &kw_s, &fmts, true)
                        })
                        .await
                        {
                            if let Some(p) = bundle.excel_path {
                                let mut w = ex_s.write().await;
                                if w.is_none() {
                                    *w = Some(p.clone());
                                }
                                let _ = app_s.emit(
                                    "autosave-done",
                                    AutosaveEvent {
                                        path: p,
                                        count: done_now,
                                    },
                                );
                            }
                        }
                    });
                }
            }
        }

        if cancel_flag.load(Ordering::Relaxed) {
            let ui_partial: Vec<UiJournalItem> =
                enriched_list.iter().map(UiJournalItem::from).collect();
            {
                let mut cur_w = current_results.write().await;
                *cur_w = enriched_list;
            }
            emit_log(&app, "Pencarian dihentikan; hasil parsial disimpan.");
            let _ = app.emit(
                "scrape-complete",
                ScrapeCompleteEvent {
                    total_count: ui_partial.len(),
                    results: ui_partial,
                    excel_path: String::new(),
                    duration_sec: started_at.elapsed().as_secs_f64(),
                    cancelled: true,
                },
            );
            is_running.store(false, Ordering::SeqCst);
            return;
        }

        // Apply month & APC filters cleanly after deep crawl
        let mut survivors: Vec<JournalRecord> = enriched_list
            .into_iter()
            .filter(|r| month::lolos_filter_bulan(&r.publish_month_nums, &selected_months, params.include_unknown))
            .filter(|r| filter_apc_tier(r, &params))
            .collect();

        // Sort highest relevance + SINTA + affordability first, then cap to target
        survivors.sort_by(|a, b| {
            b.match_score
                .total_cmp(&a.match_score)
                .then_with(|| a.biaya_sort_idr().cmp(&b.biaya_sort_idr()))
        });
        survivors.truncate(target);

        // ── PHASE 3: Selective Ollama AI Enrichment (8-item cap, 45s budget) ───────
        if params.use_ai && !survivors.is_empty() && !cancel_flag.load(Ordering::Relaxed) {
            emit_log(
                &app,
                "Fase 3: Validasi cerdas AI selektif (maks 8 jurnal prioritas, batas 45 dtk)...",
            );
            let kpi = compute_kpi(&survivors);
            ollama::enrich_records_selective(
                &client,
                &mut survivors,
                params.ai_model.as_deref(),
                cancel_flag.clone(),
                |d, t, title| {
                    emit_progress(
                        &app,
                        "fase3_ai",
                        d,
                        t,
                        title,
                        &format!("Validasi AI Ollama ({d}/{t} jurnal prioritas)..."),
                        started_at,
                        kpi,
                    );
                },
            )
            .await;
        }

        // Recompute final match score after any AI focus enrichment
        if params.match_enabled {
            let ms_ref = manuscript_guard.read().await;
            for rec in &mut survivors {
                let (sc, lbl, rsn) =
                    doc_matcher::compute_match_score(rec, &keywords, ms_ref.as_ref());
                rec.match_score = sc;
                rec.match_label = lbl;
                rec.match_reasons = rsn;
            }
        }

        // ── PHASE 4: Sync-Offloaded Exports (.xlsx, .md, .csv, _LIST.txt) ──────────
        let kpi_final = compute_kpi(&survivors);
        emit_progress(
            &app,
            "fase4_export",
            survivors.len(),
            survivors.len(),
            "Menulis laporan Excel, Markdown & Daftar WA...",
            "Menyimpan hasil ke folder output...",
            started_at,
            kpi_final,
        );

        let export_recs = survivors.clone();
        let kw_exp = kw_label.clone();
        let fmts_exp = params.formats.clone();
        let excel_path = match tokio::task::spawn_blocking(move || {
            exporter::write_all_exports_sync(&export_recs, &kw_exp, &fmts_exp, false)
        })
        .await
        {
            Ok(Ok(bundle)) => {
                if let Some(ref p) = bundle.excel_path {
                    emit_log(&app, format!("📊 Excel disimpan: {p}"));
                }
                if let Some(ref p) = bundle.markdown_path {
                    emit_log(&app, format!("📝 Markdown disimpan: {p}"));
                }
                bundle.excel_path.unwrap_or_default()
            }
            _ => String::new(),
        };

        {
            let mut ex_w = last_excel_path.write().await;
            *ex_w = (!excel_path.is_empty()).then_some(excel_path.clone());
        }

        let ui_results: Vec<UiJournalItem> = survivors.iter().map(UiJournalItem::from).collect();
        {
            let mut cur_w = current_results.write().await;
            *cur_w = survivors;
        }

        let duration_sec = started_at.elapsed().as_secs_f64();
        emit_log(
            &app,
            format!(
                "✅ Selesai dalam {:.1} dtk — {} jurnal valid siap ditinjau.",
                duration_sec,
                ui_results.len()
            ),
        );

        let _ = app.emit(
            "scrape-complete",
            ScrapeCompleteEvent {
                total_count: ui_results.len(),
                results: ui_results,
                excel_path,
                duration_sec,
                cancelled: false,
            },
        );
        is_running.store(false, Ordering::SeqCst);
    });

    Ok(StartSearchResp {
        ok: true,
        run_id,
        message: "Pencarian dimulai".to_string(),
    })
}

#[tauri::command]
pub async fn stop_search(state: State<'_, AppState>) -> Result<OkResp, JsfsError> {
    state.cancel_flag.store(true, Ordering::SeqCst);
    Ok(OkResp { ok: true })
}

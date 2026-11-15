//! 4-Phase Async Search Pipeline (`start_search` & `stop_search`) implementing Filter-Before-Deep,
//! Semaphore(32) concurrent OJS crawl, incremental auto-save every 50 journals, and instant cancel.

use crate::ai::{cloud, ollama};
use crate::config::{effective_max_workers, AUTOSAVE_INTERVAL_ITEMS};
use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::error::JsfsError;
use crate::exporter;
use crate::models::{JournalRecord, ProgressEvent, SearchParams, UiJournalItem};
use crate::parser::{doc_matcher, month};
use crate::scraper::{dedup, garuda, international, ojs_deep, sinta, DomainBreaker};
use chrono::{Datelike, Local};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

pub struct StartSearchResp {
    pub ok: bool,
    pub message: String,
}

fn emit_log(sink: &EventSink, msg: impl Into<String>) {
    sink.send(EngineEvent::LogLine(msg.into()));
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
    sink: &EventSink,
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

    sink.send(EngineEvent::Progress(ProgressEvent {
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
    }));
}

fn filter_accreditation_tier(
    rec: &JournalRecord,
    allowed_ranks: &[u8],
    include_non_sinta_garuda: bool,
    allowed_quartiles: &[String],
    filter_quartiles: bool,
) -> bool {
    if rec.is_international {
        if !filter_quartiles && allowed_quartiles.is_empty() {
            return true;
        }
        return rec.quartile.as_deref().is_some_and(|q| {
            allowed_quartiles
                .iter()
                .any(|aq| aq.eq_ignore_ascii_case(q))
        });
    }
    if allowed_ranks.is_empty() {
        return true;
    }
    match rec.sinta_rank_num() {
        Some(r) => allowed_ranks.contains(&r),
        None => include_non_sinta_garuda,
    }
}

fn filter_apc_tier(rec: &JournalRecord, params: &SearchParams) -> bool {
    match rec.apc_category() {
        "free" => params.min_rp <= 0 && params.max_rp >= 0,
        "unknown" => !params.only_free && params.include_na,
        "paid" => {
            if params.only_free {
                return false;
            }
            let idr = rec.publication_fee_idr.unwrap_or(i64::MAX);
            idr >= params.min_rp && idr <= params.max_rp
        }
        _ => false,
    }
}

/// Strict post-crawl & post-AI filter gate enforcing `sinta`, `quartiles`, `include_non_sinta_garuda`,
/// `exclude_dormant`, `only_free`, `min_rp..=max_rp`, `include_na`, and publish months.
pub fn apply_strict_filters(records: &mut Vec<JournalRecord>, params: &SearchParams) {
    let allowed_ranks = params.normalized_sinta_ranks();
    let allowed_quartiles = params.normalized_quartiles();
    let selected_months = params.normalized_months();
    records.retain(|r| {
        filter_accreditation_tier(
            r,
            &allowed_ranks,
            params.include_non_sinta_garuda,
            &allowed_quartiles,
            params.filter_quartiles,
        ) && (!params.exclude_dormant || !r.is_dormant)
            && month::lolos_filter_bulan(
                &r.publish_month_nums,
                &selected_months,
                params.include_unknown,
            )
            && filter_apc_tier(r, params)
    });
}

pub fn start_search(
    engine: Arc<EngineState>,
    sink: EventSink,
    params: SearchParams,
) -> Result<StartSearchResp, JsfsError> {
    if engine.is_running.load(Ordering::SeqCst) {
        return Ok(StartSearchResp {
            ok: false,
            message: "Pencarian sedang berjalan.".to_string(),
        });
    }

    let keywords = params.normalized_keywords();
    if keywords.is_empty() {
        return Err(JsfsError::InvalidInput(
            "Kata kunci pencarian wajib diisi.".to_string(),
        ));
    }

    engine.cancel_flag.store(false, Ordering::SeqCst);
    engine.is_running.store(true, Ordering::SeqCst);

    let client = engine.http_client.clone();
    let cache = engine.cache.clone();
    let cancel_flag = engine.cancel_flag.clone();
    let is_running = engine.is_running.clone();
    let current_results = engine.current_results.clone();
    let last_excel_path = engine.last_excel_path.clone();
    let manuscript_guard = engine.manuscript.clone();
    let matcher_cfg_guard = engine.matcher_config.clone();
    let app = sink;

    engine.runtime.spawn(async move {
        let started_at = Instant::now();
        let target = params.effective_target();
        let sinta_ranks = params.normalized_sinta_ranks();
        let allowed_quartiles = params.normalized_quartiles();
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

        // PHASE 1: Portal Scraping (Garuda + SINTA concurrent per keyword)
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
                            "⚠️ SINTA membatasi akses HTTP paralel (Cloudflare); hasil Garuda/Internasional tetap diproses.",
                        );
                    }
                    rows
                });
            }
            if params.openalex {
                let c = client.clone();
                let kw_c = kw.clone();
                let cf = cancel_flag.clone();
                let app_c = app.clone();
                let rate = params.usd_to_idr;
                portal_tasks.spawn(async move {
                    international::scrape_openalex(&c, &kw_c, rate, cf, |d, t, msg| {
                        emit_progress(
                            &app_c,
                            "fase1_portal",
                            d,
                            t,
                            msg,
                            "Mengakses OpenAlex Q1–Q4 API...",
                            started_at,
                            (0, 0, 0, 0),
                        );
                    })
                    .await
                });
            }
            if params.doaj {
                let c = client.clone();
                let kw_c = kw.clone();
                let cf = cancel_flag.clone();
                let app_c = app.clone();
                let rate = params.usd_to_idr;
                portal_tasks.spawn(async move {
                    international::scrape_doaj(&c, &kw_c, rate, cf, |d, t, msg| {
                        emit_progress(
                            &app_c,
                            "fase1_portal",
                            d,
                            t,
                            msg,
                            "Mengakses DOAJ Diamond OA API...",
                            started_at,
                            (0, 0, 0, 0),
                        );
                    })
                    .await
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
            app.send(EngineEvent::SearchComplete {
                results: vec![],
                excel_path: String::new(),
                duration_sec: started_at.elapsed().as_secs_f64(),
                cancelled: true,
            });
            is_running.store(false, Ordering::SeqCst);
            return;
        }

        // Deduplicate cross-source and apply Filter-Before-Deep on SINTA / Q1-Q4 tiers
        let deduped = dedup::deduplicate_and_merge(raw_candidates);
        let mut deep_pool: Vec<JournalRecord> = deduped
            .into_iter()
            .filter(|r| {
                filter_accreditation_tier(
                    r,
                    &sinta_ranks,
                    params.include_non_sinta_garuda,
                    &allowed_quartiles,
                    params.filter_quartiles,
                )
            })
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

        // PHASE 2: Concurrent OJS Deep Crawl (CPU-adaptive semaphore) + Auto-Save
        let total_deep = deep_pool.len();
        let sem = Arc::new(Semaphore::new(effective_max_workers()));
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
                if rec.is_international && rec.garuda_url.is_empty() && rec.sinta_url.is_empty() {
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
                    app.send(EngineEvent::ResultBatch(std::mem::take(&mut pending_batch)));
                }

                // Incremental auto-save every 50 enriched records
                if done_now.is_multiple_of(AUTOSAVE_INTERVAL_ITEMS) && !enriched_list.is_empty() {
                    let snap = enriched_list.clone();
                    let kw_s = kw_label.clone();
                    let fmts = params.formats.clone();
                    let app_s = app.clone();
                    let ex_s = last_excel_path.clone();
                    tokio::spawn(async move {
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
                                app_s.send(EngineEvent::AutosaveDone {
                                    path: p,
                                    count: done_now,
                                });
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
            app.send(EngineEvent::SearchComplete {
                results: ui_partial,
                excel_path: String::new(),
                duration_sec: started_at.elapsed().as_secs_f64(),
                cancelled: true,
            });
            is_running.store(false, Ordering::SeqCst);
            return;
        }

        // Apply strict filters cleanly after deep crawl (keep small buffer above target before Phase 3 AI)
        let mut survivors = enriched_list;
        apply_strict_filters(&mut survivors, &params);

        // Sort highest relevance + affordability first, reserving buffer for post-AI re-filtering
        survivors.sort_by(|a, b| {
            b.match_score
                .total_cmp(&a.match_score)
                .then_with(|| a.biaya_sort_idr().cmp(&b.biaya_sort_idr()))
        });
        survivors.truncate(target.saturating_add(16));

        // PHASE 3: Multi-Provider Match & Selective AI Enrichment
        let active_matcher = matcher_cfg_guard.read().await.clone();
        if params.use_ai && !survivors.is_empty() && !cancel_flag.load(Ordering::Relaxed) {
            let kpi = compute_kpi(&survivors);
            match active_matcher.normalized_mode() {
                "cloud" => {
                    if active_matcher.cloud_api_key.trim().is_empty() {
                        emit_log(
                            &app,
                            "ℹ️ Mode Cloud AI dipilih namun API Key belum diisi — memakai Built-in Smart Matcher.",
                        );
                    } else {
                        let prov = active_matcher.normalized_provider().to_uppercase();
                        emit_log(
                            &app,
                            format!(
                                "Fase 3: Pengayaan AI Cloud {prov} ({}) untuk maks 8 jurnal prioritas...",
                                active_matcher.effective_cloud_model()
                            ),
                        );
                        cloud::enrich_records_cloud(
                            &client,
                            &mut survivors,
                            &active_matcher,
                            cancel_flag.clone(),
                            |d, t, title| {
                                emit_progress(
                                    &app,
                                    "fase3_ai",
                                    d,
                                    t,
                                    title,
                                    &format!("Validasi Cloud AI ({d}/{t} jurnal prioritas)..."),
                                    started_at,
                                    kpi,
                                );
                            },
                        )
                        .await;
                    }
                }
                "ollama" => {
                    emit_log(
                        &app,
                        "Fase 3: Validasi cerdas AI Ollama selektif (maks 8 jurnal prioritas, batas 45 dtk)...",
                    );
                    let ollama_m = params
                        .ai_model
                        .as_deref()
                        .or(Some(active_matcher.ollama_model.as_str()));
                    ollama::enrich_records_selective(
                        &client,
                        &mut survivors,
                        ollama_m,
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
                _ => {
                    emit_log(
                        &app,
                        "Fase 3: Built-in Smart Matcher (Bilingual BM25 + Sinonim ID/EN) aktif tanpa perlu AI eksternal.",
                    );
                }
            }
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
                if rec.ai_source.is_empty() {
                    rec.ai_source = "Built-in Smart (BM25+ID/EN)".to_string();
                }
            }
        }

        // Post-enrichment strict Budget Gate: ensure Phase 3 AI updates obey only_free / min_rp..=max_rp
        let pre_strict_len = survivors.len();
        apply_strict_filters(&mut survivors, &params);
        if survivors.len() < pre_strict_len {
            let dropped = pre_strict_len - survivors.len();
            emit_log(
                &app,
                format!("🛡️ Budget Gate pasca-AI: {dropped} jurnal disaring karena di luar batas biaya/filter."),
            );
        }
        survivors.sort_by(|a, b| {
            b.match_score
                .total_cmp(&a.match_score)
                .then_with(|| a.biaya_sort_idr().cmp(&b.biaya_sort_idr()))
        });
        survivors.truncate(target);

        // PHASE 4: Sync-Offloaded Exports (.xlsx, .md, .csv, _LIST.txt)
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

        app.send(EngineEvent::SearchComplete {
            results: ui_results,
            excel_path,
            duration_sec,
            cancelled: false,
        });
        is_running.store(false, Ordering::SeqCst);
    });

    Ok(StartSearchResp {
        ok: true,
        message: "Pencarian dimulai".to_string(),
    })
}

pub fn stop_search(engine: &EngineState) {
    engine.cancel_flag.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rec(
        title: &str,
        apc_status: &str,
        fee_idr: Option<i64>,
        sinta: &str,
        months: Vec<u8>,
    ) -> JournalRecord {
        JournalRecord {
            title: title.to_string(),
            apc_status: apc_status.to_string(),
            publication_fee_idr: fee_idr,
            sinta_score: sinta.to_string(),
            publish_month_nums: months,
            ..Default::default()
        }
    }

    #[test]
    fn test_strict_budget_and_only_free_post_filter() {
        let free_rec = make_rec(
            "Jurnal Gratis",
            "Gratis (Rp 0)",
            Some(0),
            "Sinta 2",
            vec![3, 9],
        );
        let paid_400k = make_rec(
            "Jurbisman",
            "Rp 400.000",
            Some(400_000),
            "Sinta 2",
            vec![6, 12],
        );
        let paid_3m = make_rec(
            "JSINBIS",
            "Rp 3.000.000",
            Some(3_000_000),
            "Sinta 2",
            vec![6, 12],
        );
        let unknown_rec = make_rec(
            "Jurnal NA",
            crate::config::APC_UNKNOWN,
            None,
            "Sinta 2",
            vec![3, 9],
        );

        // 1) only_free == true MUST allow ONLY Gratis (Rp 0), rejecting paid AND unknown even when include_na == true
        let params_only_free: SearchParams = serde_json::from_value(serde_json::json!({
            "only_free": true,
            "include_na": true,
            "min_rp": 0,
            "max_rp": 1000000
        }))
        .unwrap();
        let mut list1 = vec![
            free_rec.clone(),
            paid_400k.clone(),
            paid_3m.clone(),
            unknown_rec.clone(),
        ];
        apply_strict_filters(&mut list1, &params_only_free);
        assert_eq!(list1.len(), 1);
        assert_eq!(list1[0].title, "Jurnal Gratis");

        // 2) Budget range 0..=1_000_000 with include_na == false keeps Gratis + 400k, rejects 3M & NA
        let params_budget: SearchParams = serde_json::from_value(serde_json::json!({
            "only_free": false,
            "include_na": false,
            "min_rp": 0,
            "max_rp": 1000000
        }))
        .unwrap();
        let mut list2 = vec![
            free_rec.clone(),
            paid_400k.clone(),
            paid_3m.clone(),
            unknown_rec.clone(),
        ];
        apply_strict_filters(&mut list2, &params_budget);
        let titles2: Vec<&str> = list2.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles2, vec!["Jurnal Gratis", "Jurbisman"]);

        // 3) Budget min_rp = 200_000..=1_000_000 rejects Gratis (0) and 3M, keeps only 400k
        let params_min_200k: SearchParams = serde_json::from_value(serde_json::json!({
            "only_free": false,
            "include_na": false,
            "min_rp": 200000,
            "max_rp": 1000000
        }))
        .unwrap();
        let mut list3 = vec![free_rec, paid_400k, paid_3m, unknown_rec];
        apply_strict_filters(&mut list3, &params_min_200k);
        assert_eq!(list3.len(), 1);
        assert_eq!(list3[0].title, "Jurbisman");
    }

    #[test]
    fn test_strict_sinta_filter_excludes_unranked_garuda() {
        let mut s2 = make_rec(
            "Jurnal Sinta 2",
            "Gratis (Rp 0)",
            Some(0),
            "Sinta 2",
            vec![3, 9],
        );
        s2.source_portal = "SINTA + Garuda".to_string();

        let mut s5 = make_rec(
            "Jurnal Sinta 5",
            "Gratis (Rp 0)",
            Some(0),
            "Sinta 5",
            vec![3, 9],
        );
        s5.source_portal = "SINTA".to_string();

        let mut garuda_only = make_rec(
            "Jurnal Garuda Non-SINTA",
            "Gratis (Rp 0)",
            Some(0),
            crate::config::INFO_NOT_FOUND,
            vec![3, 9],
        );
        garuda_only.source_portal = "Garuda".to_string();

        let mut dormant_s2 = make_rec(
            "Jurnal Dorman 2020",
            "Gratis (Rp 0)",
            Some(0),
            "Sinta 2",
            vec![3],
        );
        dormant_s2.is_dormant = true;
        dormant_s2.last_issue_year = Some(2020);

        let params_strict: SearchParams = serde_json::from_value(serde_json::json!({
            "sinta": [1, 2, 3],
            "include_non_sinta_garuda": false,
            "exclude_dormant": true
        }))
        .unwrap();

        let mut pool = vec![
            s2.clone(),
            s5.clone(),
            garuda_only.clone(),
            dormant_s2.clone(),
        ];
        apply_strict_filters(&mut pool, &params_strict);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool[0].title, "Jurnal Sinta 2");

        // When include_non_sinta_garuda = true and exclude_dormant = false, both unranked Garuda and dormant S2 are retained
        let params_relaxed: SearchParams = serde_json::from_value(serde_json::json!({
            "sinta": [1, 2, 3],
            "include_non_sinta_garuda": true,
            "exclude_dormant": false
        }))
        .unwrap();
        let mut pool2 = vec![s2, s5, garuda_only, dormant_s2];
        apply_strict_filters(&mut pool2, &params_relaxed);
        let titles: Vec<&str> = pool2.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(
            titles,
            vec![
                "Jurnal Sinta 2",
                "Jurnal Garuda Non-SINTA",
                "Jurnal Dorman 2020"
            ]
        );
    }
}

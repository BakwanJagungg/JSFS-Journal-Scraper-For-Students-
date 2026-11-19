//! Application Settings page: instant ID/EN language switcher, theme selector, default portals, and USD-IDR rate.

use crate::config::APP_VERSION;
use crate::engine::file_ops::open_url_with_feedback;
use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::ui::i18n::{save_app_settings, t, AppLanguage, AppSettings, DisplayCurrency};
use crate::ui::state::{parse_idr_input, FilterSettings};
use crate::ui::theme::{
    accent_button, apply_theme, card, card_divider, page_header, pill_badge, secondary_button,
    toggle_row, Palette, ThemeMode,
};
use egui::{Align2, Color32, CornerRadius, Frame, Margin, RichText, ScrollArea, Sense, Stroke, Ui};
use std::sync::Arc;
use std::time::Duration;

fn option_pill(ui: &mut Ui, p: &Palette, active: bool, label: &str) -> bool {
    let (fg, fill, border) = if active {
        (Color32::WHITE, p.accent, p.accent)
    } else {
        (p.text, p.control_bg, p.line)
    };
    let resp = Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(12.5).strong().color(fg));
        })
        .response
        .interact(Sense::click());
    resp.clicked()
}

#[allow(clippy::too_many_arguments)]
pub fn render_settings_page(
    ui: &mut Ui,
    p: &Palette,
    app_settings: &mut AppSettings,
    theme_mode: &mut ThemeMode,
    filters: &mut FilterSettings,
    usd_rate_text: &mut String,
    fx_updating: &mut bool,
    app_state: &mut crate::ui::state::AppState,
    engine: &Arc<EngineState>,
    sink: &EventSink,
) {
    let txt = t(app_settings.language);
    page_header(ui, p, txt.settings_title, txt.settings_subtitle);

    ScrollArea::vertical().show(ui, |ui| {
        // Card 1: Language & Appearance
        card(ui, p, |ui| {
            ui.label(
                RichText::new(txt.settings_lang_card)
                    .size(13.5)
                    .strong()
                    .color(p.text),
            );
            card_divider(ui, p);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 260.0).max(160.0));
                    ui.label(
                        RichText::new(txt.settings_lang_label)
                            .size(13.0)
                            .color(p.text),
                    );
                    ui.label(
                        RichText::new(txt.settings_lang_hint)
                            .size(11.5)
                            .color(p.muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if option_pill(
                        ui,
                        p,
                        app_settings.language == AppLanguage::En,
                        "English",
                    ) && app_settings.language != AppLanguage::En
                    {
                        app_settings.language = AppLanguage::En;
                        let _ = save_app_settings(app_settings);
                        sink.send(EngineEvent::Toast(
                            t(app_settings.language).toast_lang_switched.to_string(),
                        ));
                    }
                    if option_pill(
                        ui,
                        p,
                        app_settings.language == AppLanguage::Id,
                        "Bahasa Indonesia",
                    ) && app_settings.language != AppLanguage::Id
                    {
                        app_settings.language = AppLanguage::Id;
                        let _ = save_app_settings(app_settings);
                        sink.send(EngineEvent::Toast(
                            t(app_settings.language).toast_lang_switched.to_string(),
                        ));
                    }
                });
            });

            card_divider(ui, p);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 260.0).max(160.0));
                    ui.label(
                        RichText::new(txt.settings_theme_label)
                            .size(13.0)
                            .color(p.text),
                    );
                    ui.label(
                        RichText::new(txt.settings_theme_hint)
                            .size(11.5)
                            .color(p.muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if option_pill(ui, p, *theme_mode == ThemeMode::Light, "JSFS Daylight")
                        && *theme_mode != ThemeMode::Light
                    {
                        *theme_mode = ThemeMode::Light;
                        app_settings.dark_theme = false;
                        apply_theme(ui.ctx(), &Palette::from_mode(*theme_mode));
                        let _ = save_app_settings(app_settings);
                    }
                    if option_pill(ui, p, *theme_mode == ThemeMode::Dark, "JSFS Dark")
                        && *theme_mode != ThemeMode::Dark
                    {
                        *theme_mode = ThemeMode::Dark;
                        app_settings.dark_theme = true;
                        apply_theme(ui.ctx(), &Palette::from_mode(*theme_mode));
                        let _ = save_app_settings(app_settings);
                    }
                });
            });
        });

        ui.add_space(12.0);

        // Card 2: Default Portal Sources & Currency Exchange Rate
        card(ui, p, |ui| {
            ui.label(
                RichText::new(txt.settings_portal_card)
                    .size(13.5)
                    .strong()
                    .color(p.text),
            );
            card_divider(ui, p);

            if toggle_row(
                ui,
                p,
                &mut app_settings.default_garuda,
                "Portal Garuda Kemdiktisaintek",
                if app_settings.language == AppLanguage::En {
                    "Enable Indonesian national Garuda journal discovery by default."
                } else {
                    "Aktifkan pencarian portal jurnal nasional Garuda secara default."
                },
            )
            .changed()
            {
                filters.garuda = app_settings.default_garuda;
            }
            card_divider(ui, p);

            if toggle_row(
                ui,
                p,
                &mut app_settings.default_sinta,
                "Portal SINTA Kemdiktisaintek",
                if app_settings.language == AppLanguage::En {
                    "Enable official SINTA 1–6 accreditation tiers by default."
                } else {
                    "Aktifkan pengambilan peringkat akreditasi SINTA 1–6 secara default."
                },
            )
            .changed()
            {
                filters.sinta_src = app_settings.default_sinta;
            }
            card_divider(ui, p);

            if toggle_row(
                ui,
                p,
                &mut app_settings.default_openalex,
                "OpenAlex (Jurnal Internasional Q1–Q4)",
                if app_settings.language == AppLanguage::En {
                    "Open REST API for global Q1–Q4 journals, citedness metrics, and APC."
                } else {
                    "API terbuka resmi untuk jurnal global Q1–Q4, metrik sitasi, dan APC."
                },
            )
            .changed()
            {
                filters.openalex = app_settings.default_openalex;
            }
            card_divider(ui, p);

            if toggle_row(
                ui,
                p,
                &mut app_settings.default_doaj,
                "DOAJ (Diamond OA • Gratis $0)",
                if app_settings.language == AppLanguage::En {
                    "Open REST API verifying peer-reviewed zero-APC international journals."
                } else {
                    "API resmi Directory of Open Access Journals untuk jurnal internasional gratis ($0)."
                },
            )
            .changed()
            {
                filters.doaj = app_settings.default_doaj;
            }

            card_divider(ui, p);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 260.0).max(160.0));
                    ui.label(
                        RichText::new(txt.settings_currency_label)
                            .size(13.0)
                            .color(p.text),
                    );
                    ui.label(
                        RichText::new(txt.settings_currency_hint)
                            .size(11.5)
                            .color(p.muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if option_pill(
                        ui,
                        p,
                        app_settings.display_currency == DisplayCurrency::Usd,
                        "Dollar (USD)",
                    ) && app_settings.display_currency != DisplayCurrency::Usd
                    {
                        app_settings.display_currency = DisplayCurrency::Usd;
                        filters.display_currency = DisplayCurrency::Usd;
                        let _ = save_app_settings(app_settings);
                        sink.send(EngineEvent::Toast(if app_settings.language == AppLanguage::En {
                            "APC display currency switched to Dollar (USD).".to_string()
                        } else {
                            "Mata uang biaya APC diubah ke Dollar (USD).".to_string()
                        }));
                    }
                    if option_pill(
                        ui,
                        p,
                        app_settings.display_currency == DisplayCurrency::Idr,
                        "Rupiah (IDR)",
                    ) && app_settings.display_currency != DisplayCurrency::Idr
                    {
                        app_settings.display_currency = DisplayCurrency::Idr;
                        filters.display_currency = DisplayCurrency::Idr;
                        let _ = save_app_settings(app_settings);
                        sink.send(EngineEvent::Toast(if app_settings.language == AppLanguage::En {
                            "APC display currency switched to Rupiah (IDR).".to_string()
                        } else {
                            "Mata uang biaya APC diubah ke Rupiah (IDR).".to_string()
                        }));
                    }
                });
            });

            card_divider(ui, p);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 330.0).max(160.0));
                    ui.label(
                        RichText::new(txt.settings_usd_label)
                            .size(13.0)
                            .color(p.text),
                    );
                    ui.label(
                        RichText::new(txt.settings_usd_hint)
                            .size(11.5)
                            .color(p.muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let fx_lbl = if *fx_updating {
                        if app_settings.language == AppLanguage::En {
                            "Updating..."
                        } else {
                            "Mengambil..."
                        }
                    } else if app_settings.language == AppLanguage::En {
                        "Refresh Live Rate"
                    } else {
                        "Perbarui Kurs Hari Ini"
                    };
                    if ui
                        .add_enabled_ui(!*fx_updating, |ui| secondary_button(ui, p, fx_lbl))
                        .inner
                        .on_hover_text(
                            "Ambil kurs USD->IDR terbaru (1x request 3 dtk ke open.er-api.com saat diklik)",
                        )
                        .clicked()
                    {
                        *fx_updating = true;
                        crate::engine::file_ops::fetch_usd_to_idr_async(
                            engine.clone(),
                            sink.clone(),
                        );
                    }
                    if ui
                        .add_sized(
                            [130.0, 32.0],
                            egui::TextEdit::singleline(usd_rate_text)
                                .align(Align2::RIGHT_CENTER)
                                .margin(Margin::symmetric(8, 5)),
                        )
                        .changed()
                    {
                        let parsed = parse_idr_input(usd_rate_text);
                        if parsed >= 1_000 {
                            app_settings.usd_to_idr = parsed;
                            filters.usd_to_idr = parsed;
                        }
                    }
                });
            });

            card_divider(ui, p);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 220.0).max(160.0));
                    ui.label(
                        RichText::new(if app_settings.language == AppLanguage::En {
                            "Polite Academic Bot Mode"
                        } else {
                            "Mode Bot Akademik Sopan"
                        })
                        .size(13.0)
                        .color(p.text),
                    );
                    ui.label(
                        RichText::new(if app_settings.language == AppLanguage::En {
                            "Identifies JSFS transparently via academic User-Agent and widens per-domain request spacing."
                        } else {
                            "Menggunakan User-Agent riset akademik yang santun dan memperlebar jeda antar-request domain kampus."
                        })
                        .size(11.5)
                        .color(p.muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if option_pill(
                        ui,
                        p,
                        app_settings.polite_bot_mode,
                        if app_settings.language == AppLanguage::En {
                            "Enabled"
                        } else {
                            "Aktif"
                        },
                    ) {
                        app_settings.polite_bot_mode = !app_settings.polite_bot_mode;
                        crate::scraper::set_polite_bot_mode(app_settings.polite_bot_mode);
                        let _ = save_app_settings(app_settings);
                        sink.send(EngineEvent::Toast(if app_settings.polite_bot_mode {
                            "Mode Bot Akademik Sopan diaktifkan.".to_string()
                        } else {
                            "Mode rotasi browser standar diaktifkan.".to_string()
                        }));
                    }
                });
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if accent_button(ui, p, txt.settings_save_btn).clicked() {
                    let parsed = parse_idr_input(usd_rate_text);
                    if parsed >= 1_000 {
                        app_settings.usd_to_idr = parsed;
                    }
                    let clean = app_settings.clone().normalized();
                    *app_settings = clean;
                    *usd_rate_text = app_settings.usd_to_idr.to_string();
                    crate::scraper::set_polite_bot_mode(app_settings.polite_bot_mode);
                    filters.apply_app_settings(app_settings);
                    let _ = save_app_settings(app_settings);
                    sink.send(EngineEvent::Toast(txt.toast_settings_saved.to_string()));
                }

                if secondary_button(ui, p, txt.settings_reset_btn).clicked() {
                    *app_settings = AppSettings::default();
                    *theme_mode = ThemeMode::Dark;
                    apply_theme(ui.ctx(), &Palette::from_mode(*theme_mode));
                    *usd_rate_text = app_settings.usd_to_idr.to_string();
                    crate::scraper::set_polite_bot_mode(app_settings.polite_bot_mode);
                    filters.apply_app_settings(app_settings);
                    let _ = save_app_settings(app_settings);
                    sink.send(EngineEvent::Toast(txt.toast_settings_reset.to_string()));
                }
            });
        });

        ui.add_space(12.0);

        // Card 3: Author Profile for Auto-Fill Cover Letters, Inquiry Emails & Originality Statements
        card(ui, p, |ui| {
            let is_en = app_settings.language == AppLanguage::En;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if is_en {
                        "Author Profile (Auto-Fill Letters & Submission Bundle)"
                    } else {
                        "Profil Penulis (Auto-Fill Surat Pengantar, Email & Pernyataan)"
                    })
                    .size(13.5)
                    .strong()
                    .color(p.text),
                );
                pill_badge(ui, "Lokal", p.good, p.good_soft);
            });
            card_divider(ui, p);

            let field_row = |ui: &mut Ui, label: &str, hint: &str, val: &mut String| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).size(12.5).color(p.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_sized(
                            [320.0, 28.0],
                            egui::TextEdit::singleline(val)
                                .hint_text(hint)
                                .margin(Margin::symmetric(8, 5)),
                        );
                    });
                });
                ui.add_space(6.0);
            };

            field_row(
                ui,
                if is_en { "Full Name" } else { "Nama Lengkap" },
                if is_en { "e.g., Ahmad Mahasiswa, S.Kom." } else { "Mis. Ahmad Mahasiswa" },
                &mut app_settings.author_name,
            );
            field_row(
                ui,
                "NIM / NIDN",
                "Opsional (contoh: 21051204001)",
                &mut app_settings.author_id,
            );
            field_row(
                ui,
                if is_en { "Affiliation / Department" } else { "Program Studi / Universitas" },
                if is_en { "Department & University" } else { "Mis. Teknik Informatika, Universitas ..." },
                &mut app_settings.author_affiliation,
            );
            field_row(
                ui,
                if is_en { "Corresponding Email" } else { "Email Korespondensi" },
                "nama@kampus.ac.id",
                &mut app_settings.author_email,
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if accent_button(
                    ui,
                    p,
                    if is_en {
                        "Save Author Profile"
                    } else {
                        "Simpan Profil Penulis"
                    },
                )
                .clicked()
                {
                    *app_settings = app_settings.clone().normalized();
                    let _ = save_app_settings(app_settings);
                    sink.send(EngineEvent::Toast(if is_en {
                        "Author profile saved for auto-fill templates.".to_string()
                    } else {
                        "Profil penulis disimpan ke .cache/app_settings.json.".to_string()
                    }));
                }
            });
        });

        ui.add_space(12.0);

        // Card 3: Version & Manual On-Demand GitHub Update Checker (Feature #12)
        card(ui, p, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if app_settings.language == AppLanguage::En {
                        "Version & Manual Update Check"
                    } else {
                        "Versi Aplikasi & Cek Pembaruan Manual"
                    })
                    .size(13.5)
                    .strong()
                    .color(p.text),
                );
                pill_badge(ui, &format!("v{APP_VERSION}"), p.accent, p.accent_soft);
            });
            card_divider(ui, p);

            ui.horizontal_wrapped(|ui| {
                if secondary_button(
                    ui,
                    p,
                    if app_settings.language == AppLanguage::En {
                        "Check Latest Version (GitHub)"
                    } else {
                        "Cek Versi Terbaru (GitHub)"
                    },
                )
                .on_hover_text(
                    "Pemeriksaan 1x saat diklik (timeout 3 detik, tanpa proses latar belakang)",
                )
                .clicked()
                {
                    let client = engine.http_client.clone();
                    let sink_c = sink.clone();
                    engine.runtime.spawn(async move {
                        let url = "https://api.github.com/repos/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/latest";
                        let resp = client
                            .get(url)
                            .header("Accept", "application/vnd.github+json")
                            .timeout(Duration::from_secs(3))
                            .send()
                            .await;
                        match resp {
                            Ok(r) if r.status().is_success() => {
                                if let Ok(val) = r.json::<serde_json::Value>().await {
                                    let latest_tag = val
                                        .get("tag_name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .trim_start_matches('v')
                                        .to_string();
                                    if !latest_tag.is_empty() && latest_tag != APP_VERSION {
                                        sink_c.send(EngineEvent::Toast(format!(
                                            "Versi baru v{latest_tag} tersedia di GitHub Releases (saat ini v{APP_VERSION})."
                                        )));
                                    } else {
                                        sink_c.send(EngineEvent::Toast(format!(
                                            "✅ Anda sudah menggunakan versi terbaru (v{APP_VERSION})."
                                        )));
                                    }
                                }
                            }
                            _ => {
                                sink_c.send(EngineEvent::Toast(format!(
                                    "Versi aktif: v{APP_VERSION} (gagal menghubungi GitHub API dalam 3 dtk)."
                                )));
                            }
                        }
                    });
                }

                if secondary_button(ui, p, "Buka Halaman GitHub Releases").clicked() {
                    open_url_with_feedback(
                        "https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases",
                        sink,
                    );
                }
            });
        });

        ui.add_space(12.0);

        // Card 5: Local Backup & Restore (.json)
        card(ui, p, |ui| {
            let is_en = app_settings.language == AppLanguage::En;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if is_en {
                        "Local Data Backup & Restore (.json)"
                    } else {
                        "Cadangan & Pemulihan Data Lokal (.json)"
                    })
                    .size(13.5)
                    .strong()
                    .color(p.text),
                );
                pill_badge(
                    ui,
                    if is_en { "Portable JSON" } else { "JSON Portabel" },
                    p.accent,
                    p.accent_soft,
                );
            });
            ui.label(
                RichText::new(if is_en {
                    "Export all preferences, author profiles, saved manuscripts, shortlisted journals (★), and submission tracker statuses into a single portable JSON file, or restore them onto another device."
                } else {
                    "Gabungkan seluruh pengaturan, profil penulis, riwayat naskah, daftar jurnal pilihan (★), dan status pelacak submisi ke 1 file JSON portabel, atau pulihkan saat berpindah perangkat."
                })
                .size(11.5)
                .color(p.muted),
            );
            card_divider(ui, p);

            ui.horizontal_wrapped(|ui| {
                if accent_button(
                    ui,
                    p,
                    if is_en {
                        "Export Backup (.json)"
                    } else {
                        "Ekspor Cadangan (.json)"
                    },
                )
                .on_hover_text(if is_en {
                    "Save full settings, profiles, shortlist, and tracker data into a JSON backup file"
                } else {
                    "Simpan seluruh pengaturan, profil, shortlist, dan pelacak submisi ke file cadangan JSON"
                })
                .clicked()
                {
                    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let default_name = format!("jsfs_backup_{ts}.json");
                    if let Some(dest) = rfd::FileDialog::new()
                        .set_file_name(&default_name)
                        .add_filter("JSON File (*.json)", &["json"])
                        .save_file()
                    {
                        let settings_val = serde_json::to_value(&*app_settings).unwrap_or_default();
                        let profiles_val = serde_json::to_value(&app_state.saved_profiles).unwrap_or_default();
                        let shortlisted: Vec<String> = app_state.bookmarked_ids.iter().cloned().collect();
                        let tracks: Vec<crate::cache::SubmissionTrack> = app_state.submission_tracks.values().cloned().collect();
                        let bundle = crate::config::create_backup_bundle(
                            settings_val,
                            profiles_val,
                            shortlisted,
                            app_state.saved_shortlist_items.clone(),
                            tracks,
                        );
                        match crate::config::save_backup_bundle_to_path(&bundle, &dest) {
                            Ok(_) => {
                                sink.send(EngineEvent::Toast(if is_en {
                                    format!("Backup successfully exported to: {}", dest.display())
                                } else {
                                    format!("Cadangan berhasil diekspor ke: {}", dest.display())
                                }));
                            }
                            Err(e) => {
                                sink.send(EngineEvent::Toast(format!("Gagal mengekspor cadangan: {e}")));
                            }
                        }
                    }
                }

                if secondary_button(
                    ui,
                    p,
                    if is_en {
                        "Import Backup (.json)"
                    } else {
                        "Impor Cadangan (.json)"
                    },
                )
                .on_hover_text(if is_en {
                    "Load and restore settings, profiles, and tracked journals from a JSON backup file"
                } else {
                    "Muat dan pulihkan pengaturan, profil, dan jurnal terlacak dari berkas cadangan JSON"
                })
                .clicked()
                {
                    if let Some(src) = rfd::FileDialog::new()
                        .add_filter("JSON File (*.json)", &["json"])
                        .pick_file()
                    {
                        match std::fs::read_to_string(&src) {
                            Ok(content) => match crate::config::JsfsBackupBundle::from_json_str(&content) {
                                Ok(bundle) => {
                                    // 1. Restore AppSettings
                                    if let Ok(st) = serde_json::from_value::<AppSettings>(bundle.settings) {
                                        *app_settings = st;
                                        *usd_rate_text = app_settings.usd_to_idr.to_string();
                                        filters.apply_app_settings(app_settings);
                                        let _ = crate::ui::i18n::save_app_settings(app_settings);
                                    }
                                    // 2. Restore Saved Profiles
                                    if let Ok(profs) = serde_json::from_value::<Vec<crate::ui::state::SavedSearchProfile>>(bundle.saved_profiles) {
                                        app_state.saved_profiles = profs;
                                        let _ = crate::ui::state::save_saved_profiles_to(
                                            &crate::config::saved_profiles_path(),
                                            &app_state.saved_profiles,
                                        );
                                    }
                                    // 3. Restore Saved Manuscripts
                                    if !bundle.saved_manuscripts.is_empty() {
                                        let _ = crate::config::save_saved_manuscripts(&bundle.saved_manuscripts);
                                    }
                                    // 4. Restore Shortlist IDs & Items
                                    let added_bm = bundle.shortlisted_keys.len().max(bundle.saved_shortlist.len());
                                    for k in bundle.shortlisted_keys {
                                        app_state.bookmarked_ids.insert(k);
                                    }
                                    for item in bundle.saved_shortlist {
                                        let key = crate::ui::state::AppState::bookmark_key(&item);
                                        app_state.bookmarked_ids.insert(key.clone());
                                        if !app_state.saved_shortlist_items.iter().any(|it| crate::ui::state::AppState::bookmark_key(it) == key) {
                                            app_state.saved_shortlist_items.push(item);
                                        }
                                    }
                                    let _ = crate::config::save_saved_shortlist(&app_state.saved_shortlist_items);
                                    // 5. Restore Submission Tracks
                                    let added_tr = bundle.submission_tracks.len();
                                    if let Ok(guard) = engine.cache.lock() {
                                        if let Some(ref db) = *guard {
                                            for tr in &bundle.submission_tracks {
                                                let _ = db.upsert_submission_track(tr);
                                            }
                                        }
                                    }
                                    for tr in bundle.submission_tracks {
                                        app_state.submission_tracks.insert(tr.journal_key.clone(), tr);
                                    }
                                    sink.send(EngineEvent::Toast(if is_en {
                                        format!("Backup restored! ({added_bm} starred, {added_tr} tracked journals)")
                                    } else {
                                        format!("Cadangan berhasil dipulihkan! ({added_bm} bintang ★, {added_tr} status pelacak)")
                                    }));
                                }
                                Err(e) => {
                                    sink.send(EngineEvent::Toast(format!("File JSON cadangan tidak valid: {e}")));
                                }
                            },
                            Err(e) => {
                                sink.send(EngineEvent::Toast(format!("Gagal membaca berkas cadangan: {e}")));
                            }
                        }
                    }
                }
            });
        });

        ui.add_space(24.0);
    });
}

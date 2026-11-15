//! Multi-provider Match Engine settings page (Built-in BM25 Default, Cloud AI API, Ollama Local).

use crate::config::default_model_for_provider;
use crate::engine::ai_ops::{
    install_ollama_async, launch_ollama_async, pull_ai_model_async, refresh_ai_status_async,
    save_matcher_config_async, test_cloud_api_async, AiStatusExtended, SaveMatcherConfigInput,
};
use crate::engine::{EngineState, EventSink};
use crate::ui::i18n::{t, AppLanguage};
use crate::ui::state::MatcherFormState;
use crate::ui::theme::{
    accent_button, card, card_divider, page_header, pill_badge, secondary_button, Palette,
};
use egui::{CornerRadius, Frame, Margin, ProgressBar, RichText, ScrollArea, Sense, Stroke, Ui};
use std::sync::Arc;

fn mode_card(
    ui: &mut Ui,
    p: &Palette,
    selected: bool,
    title: &str,
    badge: &str,
    desc: &str,
) -> bool {
    let fill = if selected { p.accent_soft } else { p.card_bg };
    let border = if selected { p.accent } else { p.line };
    let mut radio_clicked = false;
    let resp = Frame::new()
        .fill(fill)
        .stroke(Stroke::new(if selected { 1.5 } else { 1.0 }, border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                radio_clicked = ui.radio(selected, "").clicked();
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(title).size(13.5).strong().color(p.text));
                        pill_badge(
                            ui,
                            badge,
                            if selected { p.accent } else { p.muted },
                            if selected {
                                p.accent_soft
                            } else {
                                p.control_bg
                            },
                        );
                    });
                    ui.label(RichText::new(desc).size(12.0).color(p.muted));
                });
            });
        })
        .response
        .interact(Sense::click());
    resp.clicked() || radio_clicked
}

fn build_save_input(form: &MatcherFormState) -> SaveMatcherConfigInput {
    SaveMatcherConfigInput {
        mode: form.mode.clone(),
        cloud_provider: Some(form.cloud_provider.clone()),
        cloud_api_key: (!form.cloud_api_key_input.trim().is_empty())
            .then(|| form.cloud_api_key_input.trim().to_string()),
        clear_api_key: form.clear_api_key,
        cloud_model: Some(form.cloud_model.clone()),
        cloud_base_url: Some(form.cloud_base_url.clone()),
        ollama_model: Some(form.ollama_model.clone()),
    }
}

pub fn render_match_engine_page(
    ui: &mut Ui,
    p: &Palette,
    lang: AppLanguage,
    form: &mut MatcherFormState,
    status: &AiStatusExtended,
    engine: &Arc<EngineState>,
    sink: &EventSink,
) {
    let txt = t(lang);
    page_header(ui, p, txt.match_title, txt.match_subtitle);

    ScrollArea::vertical().show(ui, |ui| {
        if mode_card(
            ui,
            p,
            form.mode == "builtin",
            "Built-in Smart Matcher (Bilingual BM25 + Sinonim ID/EN)",
            "Default • Gratis • Zero-Setup",
            "Bekerja instan secara offline tanpa perlu API Key maupun Ollama. Mengerti padanan akademik Indonesia–Inggris seperti UMKM <-> SME, pembelajaran <-> learning, dan sistem informasi <-> information system.",
        ) {
            form.mode = "builtin".to_string();
        }

        ui.add_space(10.0);

        if mode_card(
            ui,
            p,
            form.mode == "cloud",
            "Cloud AI API (Groq / Google Gemini / OpenAI-Compatible)",
            "API Key Opsional",
            "Memakai LLM cloud super cepat untuk memverifikasi teks APC terselubung & kecocokan Focus-Scope secara semantik (API Key disimpan aman secara lokal).",
        ) {
            form.mode = "cloud".to_string();
        }

        if form.mode == "cloud" {
            ui.add_space(6.0);
            card(ui, p, |ui| {
                ui.columns(2, |cols| {
                    cols[0].label(RichText::new("Provider Cloud").size(12.0).color(p.muted));
                    let prev_prov = form.cloud_provider.clone();
                    egui::ComboBox::from_id_salt("cloud_provider_combo")
                        .width(cols[0].available_width())
                        .selected_text(match form.cloud_provider.as_str() {
                            "gemini" => "Google Gemini",
                            "openai_compatible" => "OpenAI-Compatible",
                            _ => "Groq (Llama 3.1)",
                        })
                        .show_ui(&mut cols[0], |ui| {
                            ui.selectable_value(&mut form.cloud_provider, "groq".to_string(), "Groq (Llama 3.1)");
                            ui.selectable_value(&mut form.cloud_provider, "gemini".to_string(), "Google Gemini");
                            ui.selectable_value(
                                &mut form.cloud_provider,
                                "openai_compatible".to_string(),
                                "OpenAI-Compatible (Custom URL)",
                            );
                        });
                    if form.cloud_provider != prev_prov {
                        form.cloud_model =
                            default_model_for_provider(&form.cloud_provider).to_string();
                    }

                    cols[1].label(RichText::new("Nama Model").size(12.0).color(p.muted));
                    cols[1].add_sized(
                        [cols[1].available_width(), 28.0],
                        egui::TextEdit::singleline(&mut form.cloud_model),
                    );
                });

                if form.cloud_provider == "openai_compatible" {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Base URL Endpoint").size(12.0).color(p.muted));
                    ui.add_sized(
                        [ui.available_width(), 28.0],
                        egui::TextEdit::singleline(&mut form.cloud_base_url)
                            .hint_text("https://api.openai.com/v1"),
                    );
                }

                card_divider(ui, p);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("API Key").size(12.0).color(p.muted));
                    if status.cloud_has_key && !form.clear_api_key {
                        pill_badge(
                            ui,
                            &format!("Tersimpan: {}", status.cloud_key_masked),
                            p.good,
                            p.good_soft,
                        );
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let key_w = (ui.available_width() - 210.0).max(160.0);
                    ui.add_sized(
                        [key_w, 28.0],
                        egui::TextEdit::singleline(&mut form.cloud_api_key_input)
                            .password(true)
                            .hint_text("Tempel API Key baru Anda di sini..."),
                    );
                    if secondary_button(ui, p, "Tes Koneksi").clicked() {
                        form.cloud_test_result = Some((true, "Menghubungi endpoint Cloud AI...".to_string()));
                        test_cloud_api_async(engine.clone(), sink.clone(), build_save_input(form));
                    }
                    if status.cloud_has_key && secondary_button(ui, p, "Hapus Key").clicked() {
                        form.clear_api_key = true;
                        form.cloud_api_key_input.clear();
                    }
                });

                if let Some((ok, msg)) = &form.cloud_test_result {
                    ui.add_space(6.0);
                    pill_badge(
                        ui,
                        msg,
                        if *ok { p.good } else { p.bad },
                        if *ok { p.good_soft } else { p.bad_soft },
                    );
                }
            });
        }

        ui.add_space(10.0);

        if mode_card(
            ui,
            p,
            form.mode == "ollama",
            "Ollama Lokal (Power-User • localhost:11434)",
            "100% Privat di PC",
            "Menjalankan model LLM lokal seperti qwen2.5:1.5b di komputer Anda tanpa batas kuota API.",
        ) {
            form.mode = "ollama".to_string();
        }

        if form.mode == "ollama" {
            ui.add_space(6.0);
            card(ui, p, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Status Layanan:").size(12.5).color(p.text));
                    if status.running {
                        pill_badge(ui, "Ollama Aktif", p.good, p.good_soft);
                    } else if status.installed {
                        pill_badge(ui, "Terpasang (Belum Jalan)", p.warn, p.warn_soft);
                    } else {
                        pill_badge(ui, "Belum Terpasang", p.bad, p.bad_soft);
                    }

                    if status.model_installed {
                        pill_badge(
                            ui,
                            &format!("Model {} Siap", form.ollama_model),
                            p.good,
                            p.good_soft,
                        );
                    } else {
                        pill_badge(ui, "Model Belum Diunduh", p.warn, p.warn_soft);
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Model Ollama:").size(12.0).color(p.muted));
                    ui.add_sized([220.0, 28.0], egui::TextEdit::singleline(&mut form.ollama_model));
                });

                if form.pull_running || !form.pull_message.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new(&form.pull_message).size(11.5).color(p.muted));
                    ui.add(
                        ProgressBar::new((form.pull_percent as f32 / 100.0).clamp(0.0, 1.0))
                            .fill(p.good)
                            .desired_height(6.0),
                    );
                }

                card_divider(ui, p);
                ui.horizontal_wrapped(|ui| {
                    if secondary_button(ui, p, "Cek Ulang").clicked() {
                        refresh_ai_status_async(engine.clone(), sink.clone());
                    }
                    if secondary_button(ui, p, "Jalankan Ollama").clicked() {
                        launch_ollama_async(engine.clone(), sink.clone());
                    }
                    if secondary_button(ui, p, "Unduh Ollama").clicked() {
                        install_ollama_async(engine.clone(), sink.clone());
                    }
                    if secondary_button(ui, p, "Unduh Model AI").clicked() {
                        pull_ai_model_async(
                            engine.clone(),
                            sink.clone(),
                            Some(form.ollama_model.clone()),
                        );
                    }
                });
            });
        }

        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            if accent_button(ui, p, "Simpan Pengaturan Match Engine").clicked() {
                save_matcher_config_async(engine.clone(), sink.clone(), build_save_input(form));
            }
            let selected_mode_label = match form.mode.as_str() {
                "cloud" => {
                    let prov = match form.cloud_provider.as_str() {
                        "gemini" => "Google Gemini",
                        "openai_compatible" => "OpenAI-Compatible",
                        _ => "Groq",
                    };
                    format!("Mode dipilih: Cloud AI ({prov})")
                }
                "ollama" => format!("Mode dipilih: Ollama Lokal ({})", form.ollama_model),
                _ => "Mode dipilih: Built-in Smart".to_string(),
            };
            let is_unsaved = form.mode != status.matcher_mode;
            let status_color = if is_unsaved { p.warn } else { p.accent };
            let status_bg = if is_unsaved { p.warn_soft } else { p.accent_soft };
            let full_badge = if is_unsaved {
                format!("{selected_mode_label} • Belum disimpan")
            } else {
                selected_mode_label
            };
            pill_badge(ui, &full_badge, status_color, status_bg);
        });
    });
}

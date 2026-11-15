//! Multi-provider Match Engine status, config persistence, Cloud API test, and optional Ollama management.

use crate::ai::{cloud, ollama, GLOBAL_AI_BREAKER};
use crate::config::{
    cache_dir, default_model_for_provider, mask_api_key, save_matcher_config_to_disk,
    MatcherConfig, DEFAULT_AI_MODEL, OLLAMA_BASE_URL,
};
use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::error::JsfsError;
use futures_util::StreamExt;
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct SaveMatcherConfigInput {
    pub mode: String,
    pub cloud_provider: Option<String>,
    pub cloud_api_key: Option<String>,
    pub clear_api_key: bool,
    pub cloud_model: Option<String>,
    pub cloud_base_url: Option<String>,
    pub ollama_model: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AiStatusExtended {
    pub installed: bool,
    pub running: bool,
    pub models: Vec<String>,
    pub recommended_model: String,
    pub circuit_open: bool,
    pub model: String,
    pub model_installed: bool,
    pub task_running: bool,
    pub task_percent: f64,
    pub task_message: String,
    pub cache_count: usize,
    pub matcher_mode: String,
    pub cloud_provider: String,
    pub cloud_model: String,
    pub cloud_base_url: String,
    pub cloud_has_key: bool,
    pub cloud_key_masked: String,
    pub ollama_model: String,
    pub active_engine_label: String,
}

#[derive(Debug, Clone)]
pub struct AiActionResp {
    pub ok: bool,
    pub message: String,
}

fn compute_engine_label(cfg: &MatcherConfig, ollama_running: bool) -> String {
    match cfg.normalized_mode() {
        "cloud" => {
            if cfg.cloud_api_key.trim().is_empty() {
                "Match Engine: Cloud (Perlu API Key)".to_string()
            } else {
                let prov = match cfg.normalized_provider() {
                    "gemini" => "Gemini",
                    "openai_compatible" => "Cloud OpenAI",
                    _ => "Groq",
                };
                format!("Match Engine: Cloud ({prov})")
            }
        }
        "ollama" => {
            if ollama_running {
                "Match Engine: Ollama Aktif".to_string()
            } else {
                "Match Engine: Ollama Off".to_string()
            }
        }
        _ => "Match Engine: Built-in Smart".to_string(),
    }
}

pub async fn build_ai_status(engine: &EngineState) -> AiStatusExtended {
    let cfg = engine.matcher_config.read().await.clone();
    let st = ollama::get_ai_status(&engine.http_client).await;

    let rec = if cfg.ollama_model.trim().is_empty() {
        DEFAULT_AI_MODEL.to_string()
    } else {
        cfg.ollama_model.clone()
    };
    let base = rec.split(':').next().unwrap_or(&rec).to_ascii_lowercase();
    let model_installed = st.models.iter().any(|m| {
        let low = m.to_ascii_lowercase();
        low == rec.to_ascii_lowercase() || low.starts_with(&format!("{base}:"))
    });

    let cache_count = engine
        .cache
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|db| db.count_valid().ok()))
        .unwrap_or(0);

    let mode = cfg.normalized_mode().to_string();
    let cloud_provider = cfg.normalized_provider().to_string();
    let key = cfg.cloud_api_key.trim();
    let active_engine_label = compute_engine_label(&cfg, st.running);

    AiStatusExtended {
        installed: st.installed,
        running: st.running,
        models: st.models,
        recommended_model: rec.clone(),
        circuit_open: GLOBAL_AI_BREAKER.is_open(),
        model: rec.clone(),
        model_installed,
        task_running: false,
        task_percent: 0.0,
        task_message: String::new(),
        cache_count,
        matcher_mode: mode,
        cloud_provider,
        cloud_model: cfg.effective_cloud_model(),
        cloud_base_url: cfg.cloud_base_url.clone(),
        cloud_has_key: !key.is_empty(),
        cloud_key_masked: mask_api_key(key),
        ollama_model: rec,
        active_engine_label,
    }
}

pub fn refresh_ai_status_async(engine: Arc<EngineState>, sink: EventSink) {
    let rt = engine.runtime.handle().clone();
    rt.spawn(async move {
        let status = build_ai_status(&engine).await;
        sink.send(EngineEvent::AiStatusUpdated(status));
    });
}

pub fn save_matcher_config_async(
    engine: Arc<EngineState>,
    sink: EventSink,
    input: SaveMatcherConfigInput,
) {
    let rt = engine.runtime.handle().clone();
    rt.spawn(async move {
        let res: Result<(), JsfsError> = async {
            let mut cfg = engine.matcher_config.write().await;
            cfg.mode = match input.mode.trim().to_ascii_lowercase().as_str() {
                "cloud" => "cloud".to_string(),
                "ollama" => "ollama".to_string(),
                _ => "builtin".to_string(),
            };
            if let Some(p) = input.cloud_provider {
                let norm_p = match p.trim().to_ascii_lowercase().as_str() {
                    "gemini" => "gemini",
                    "openai" | "openai_compatible" | "openai-compatible" => "openai_compatible",
                    _ => "groq",
                };
                cfg.cloud_provider = norm_p.to_string();
            }
            if input.clear_api_key {
                cfg.cloud_api_key.clear();
            } else if let Some(key) = input.cloud_api_key {
                let trimmed = key.trim();
                if !trimmed.is_empty() {
                    cfg.cloud_api_key = trimmed.to_string();
                }
            }
            if let Some(m) = input.cloud_model {
                let tm = m.trim();
                cfg.cloud_model = if tm.is_empty() {
                    default_model_for_provider(cfg.normalized_provider()).to_string()
                } else {
                    tm.to_string()
                };
            }
            if let Some(u) = input.cloud_base_url {
                cfg.cloud_base_url = u.trim().to_string();
            }
            if let Some(om) = input.ollama_model {
                let tom = om.trim();
                if !tom.is_empty() {
                    cfg.ollama_model = tom.to_string();
                }
            }
            save_matcher_config_to_disk(&cfg)?;
            Ok(())
        }
        .await;

        let status = build_ai_status(&engine).await;
        sink.send(EngineEvent::AiStatusUpdated(status));
        match res {
            Ok(()) => sink.send(EngineEvent::Toast(
                "Pengaturan Match Engine disimpan.".to_string(),
            )),
            Err(e) => sink.send(EngineEvent::Toast(format!("Gagal menyimpan config AI: {e}"))),
        }
    });
}

pub fn test_cloud_api_async(
    engine: Arc<EngineState>,
    sink: EventSink,
    input: SaveMatcherConfigInput,
) {
    let rt = engine.runtime.handle().clone();
    rt.spawn(async move {
        let mut cfg = engine.matcher_config.read().await.clone();
        if let Some(p) = input.cloud_provider {
            cfg.cloud_provider = p;
            cfg.cloud_provider = cfg.normalized_provider().to_string();
        }
        if let Some(key) = input.cloud_api_key {
            let trimmed = key.trim();
            if !trimmed.is_empty() {
                cfg.cloud_api_key = trimmed.to_string();
            }
        }
        if let Some(m) = input.cloud_model {
            cfg.cloud_model = m.trim().to_string();
        }
        if let Some(u) = input.cloud_base_url {
            cfg.cloud_base_url = u.trim().to_string();
        }

        let resp = match cloud::test_cloud_connection(&engine.http_client, &cfg).await {
            Ok(msg) => AiActionResp {
                ok: true,
                message: msg,
            },
            Err(err) => AiActionResp {
                ok: false,
                message: err,
            },
        };
        sink.send(EngineEvent::AiActionDone(resp));
    });
}

pub fn launch_ollama_async(engine: Arc<EngineState>, sink: EventSink) {
    let rt = engine.runtime.handle().clone();
    rt.spawn(async move {
        let Some(path) = ollama::find_ollama_executable() else {
            sink.send(EngineEvent::AiActionDone(AiActionResp {
                ok: false,
                message: "Aplikasi Ollama belum terpasang di komputer ini.".to_string(),
            }));
            return;
        };

        let res = tokio::task::spawn_blocking(move || -> Result<(), JsfsError> {
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                std::process::Command::new(path)
                    .arg("serve")
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                std::process::Command::new(path).arg("serve").spawn()?;
            }
            Ok(())
        })
        .await;

        let ok = matches!(res, Ok(Ok(())));
        sink.send(EngineEvent::AiActionDone(AiActionResp {
            ok,
            message: if ok {
                "Layanan Ollama dijalankan. Tunggu 2 detik lalu klik Cek Ulang.".to_string()
            } else {
                "Gagal menjalankan layanan Ollama.".to_string()
            },
        }));
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
        let status = build_ai_status(&engine).await;
        sink.send(EngineEvent::AiStatusUpdated(status));
    });
}

pub fn install_ollama_async(engine: Arc<EngineState>, sink: EventSink) {
    let client = engine.http_client.clone();
    engine.runtime.spawn(async move {
        sink.send(EngineEvent::AiPullProgress {
            running: false,
            model_installed: false,
            task_running: true,
            task_percent: 5.0,
            task_message: "Mengunduh OllamaSetup.exe...".to_string(),
        });

        let url = "https://ollama.com/download/OllamaSetup.exe";
        let resp = match client.get(url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => {
                sink.send(EngineEvent::AiPullProgress {
                    running: false,
                    model_installed: false,
                    task_running: false,
                    task_percent: 0.0,
                    task_message: "Gagal mengunduh OllamaSetup.exe.".to_string(),
                });
                return;
            }
        };

        let total = resp.content_length().unwrap_or(0);
        let setup_dir = cache_dir().join("downloads");
        let _ = std::fs::create_dir_all(&setup_dir);
        let setup_path = setup_dir.join("OllamaSetup.exe");

        let mut downloaded: u64 = 0;
        let mut buf: Vec<u8> = Vec::new();
        let mut stream = resp.bytes_stream();
        while let Some(Ok(chunk)) = stream.next().await {
            downloaded += chunk.len() as u64;
            buf.extend_from_slice(&chunk);
            if total > 0 && (downloaded % (2 * 1024 * 1024) < chunk.len() as u64) {
                let pct = ((downloaded as f64 / total as f64) * 90.0).min(90.0);
                sink.send(EngineEvent::AiPullProgress {
                    running: false,
                    model_installed: false,
                    task_running: true,
                    task_percent: pct,
                    task_message: format!("Mengunduh Ollama ({pct:.0}%)..."),
                });
            }
        }

        let write_res = tokio::task::spawn_blocking(move || -> Result<(), JsfsError> {
            std::fs::write(&setup_path, &buf)?;
            std::process::Command::new(&setup_path).spawn()?;
            Ok(())
        })
        .await;

        let ok = matches!(write_res, Ok(Ok(())));
        sink.send(EngineEvent::AiPullProgress {
            running: false,
            model_installed: false,
            task_running: false,
            task_percent: if ok { 100.0 } else { 0.0 },
            task_message: if ok {
                "Installer Ollama dijalankan. Selesaikan wizard lalu klik Cek Ulang.".to_string()
            } else {
                "Gagal menjalankan installer Ollama.".to_string()
            },
        });
    });
}

pub fn pull_ai_model_async(engine: Arc<EngineState>, sink: EventSink, model: Option<String>) {
    let target_model = model
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_AI_MODEL.to_string());
    let client = engine.http_client.clone();

    engine.runtime.spawn(async move {
        sink.send(EngineEvent::AiPullProgress {
            running: true,
            model_installed: false,
            task_running: true,
            task_percent: 2.0,
            task_message: format!("Mengunduh model {target_model}..."),
        });

        let body = serde_json::json!({ "name": target_model, "stream": true });
        if let Ok(resp) = client
            .post(format!("{OLLAMA_BASE_URL}/api/pull"))
            .json(&body)
            .send()
            .await
        {
            let mut stream = resp.bytes_stream();
            while let Some(Ok(chunk)) = stream.next().await {
                for line in String::from_utf8_lossy(&chunk).lines() {
                    if let Ok(v) = serde_json::from_str::<Value>(line) {
                        let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
                        let done = v.get("completed").and_then(|n| n.as_f64()).unwrap_or(0.0);
                        let tot = v.get("total").and_then(|n| n.as_f64()).unwrap_or(0.0);
                        let pct = if tot > 0.0 {
                            (done / tot * 100.0).clamp(0.0, 100.0)
                        } else {
                            10.0
                        };
                        sink.send(EngineEvent::AiPullProgress {
                            running: true,
                            model_installed: status.contains("success"),
                            task_running: !status.contains("success"),
                            task_percent: pct,
                            task_message: format!("{target_model}: {status}"),
                        });
                    }
                }
            }
        }

        let final_st = ollama::get_ai_status(&client).await;
        let base = target_model
            .split(':')
            .next()
            .unwrap_or(&target_model)
            .to_ascii_lowercase();
        let installed = final_st
            .models
            .iter()
            .any(|m| m.to_ascii_lowercase().starts_with(&format!("{base}:")));
        sink.send(EngineEvent::AiPullProgress {
            running: final_st.running,
            model_installed: installed,
            task_running: false,
            task_percent: if installed { 100.0 } else { 0.0 },
            task_message: if installed {
                format!("Model {target_model} siap digunakan.")
            } else {
                format!("Unduhan {target_model} selesai/dibatalkan.")
            },
        });
    });
}

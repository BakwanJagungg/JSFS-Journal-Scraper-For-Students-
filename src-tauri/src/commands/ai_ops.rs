//! Multi-provider Match Engine status, config persistence, Cloud API test, and optional Ollama management.

use crate::ai::{cloud, ollama, GLOBAL_AI_BREAKER};
use crate::config::{
    cache_dir, default_model_for_provider, mask_api_key, save_matcher_config_to_disk,
    MatcherConfig, DEFAULT_AI_MODEL, OLLAMA_BASE_URL,
};
use crate::error::JsfsError;
use crate::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
pub struct MatcherConfigView {
    pub mode: String,
    pub cloud_provider: String,
    pub cloud_model: String,
    pub cloud_base_url: String,
    pub cloud_has_key: bool,
    pub cloud_key_masked: String,
    pub ollama_model: String,
}

impl From<&MatcherConfig> for MatcherConfigView {
    fn from(cfg: &MatcherConfig) -> Self {
        let mode = cfg.normalized_mode().to_string();
        let provider = cfg.normalized_provider().to_string();
        let key = cfg.cloud_api_key.trim();
        Self {
            mode,
            cloud_provider: provider,
            cloud_model: cfg.effective_cloud_model(),
            cloud_base_url: cfg.cloud_base_url.clone(),
            cloud_has_key: !key.is_empty(),
            cloud_key_masked: mask_api_key(key),
            ollama_model: if cfg.ollama_model.trim().is_empty() {
                DEFAULT_AI_MODEL.to_string()
            } else {
                cfg.ollama_model.clone()
            },
        }
    }
}

#[derive(Deserialize)]
pub struct SaveMatcherConfigInput {
    pub mode: String,
    #[serde(default)]
    pub cloud_provider: Option<String>,
    #[serde(default)]
    pub cloud_api_key: Option<String>,
    #[serde(default)]
    pub clear_api_key: Option<bool>,
    #[serde(default)]
    pub cloud_model: Option<String>,
    #[serde(default)]
    pub cloud_base_url: Option<String>,
    #[serde(default)]
    pub ollama_model: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct AiStatusExtended {
    pub installed: bool,
    pub running: bool,
    pub models: Vec<String>,
    pub recommended_model: String,
    pub circuit_open: bool,
    // Compatibility fields for v2.4 AI setup dialog
    pub model: String,
    pub model_installed: bool,
    pub task_running: bool,
    pub task_percent: f64,
    pub task_message: String,
    pub cache_count: usize,
    // Multi-provider v3 fields
    pub matcher_mode: String,
    pub cloud_provider: String,
    pub cloud_model: String,
    pub cloud_base_url: String,
    pub cloud_has_key: bool,
    pub cloud_key_masked: String,
    pub ollama_model: String,
    pub active_engine_label: String,
}

#[derive(Serialize)]
pub struct AiActionResp {
    pub ok: bool,
    pub progress: f64,
    pub message: String,
}

#[derive(Clone, Serialize)]
struct AiProgressEvent {
    pub running: bool,
    pub model_installed: bool,
    pub task_running: bool,
    pub task_percent: f64,
    pub task_message: String,
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

#[tauri::command]
pub async fn get_matcher_config(state: State<'_, AppState>) -> Result<MatcherConfigView, JsfsError> {
    let cfg = state.matcher_config.read().await;
    Ok(MatcherConfigView::from(&*cfg))
}

#[tauri::command]
pub async fn save_matcher_config(
    state: State<'_, AppState>,
    input: SaveMatcherConfigInput,
) -> Result<AiStatusExtended, JsfsError> {
    {
        let mut cfg = state.matcher_config.write().await;
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
        if input.clear_api_key.unwrap_or(false) {
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
    }

    get_ai_status(state).await
}

#[tauri::command]
pub async fn test_cloud_api(
    state: State<'_, AppState>,
    input: SaveMatcherConfigInput,
) -> Result<AiActionResp, JsfsError> {
    let mut cfg = state.matcher_config.read().await.clone();
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

    match cloud::test_cloud_connection(&state.http_client, &cfg).await {
        Ok(msg) => Ok(AiActionResp {
            ok: true,
            progress: 100.0,
            message: msg,
        }),
        Err(err) => Ok(AiActionResp {
            ok: false,
            progress: 0.0,
            message: err,
        }),
    }
}

#[tauri::command]
pub async fn get_ai_status(state: State<'_, AppState>) -> Result<AiStatusExtended, JsfsError> {
    let cfg = state.matcher_config.read().await.clone();
    let ollama_needed = cfg.normalized_mode() == "ollama";
    let st = if ollama_needed {
        ollama::get_ai_status(&state.http_client).await
    } else {
        // Non-blocking fast check when not actively in Ollama mode (still checks localhost quickly)
        ollama::get_ai_status(&state.http_client).await
    };

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

    let cache_count = state
        .cache
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|db| db.count_valid().ok()))
        .unwrap_or(0);

    let view = MatcherConfigView::from(&cfg);
    let active_engine_label = compute_engine_label(&cfg, st.running);

    Ok(AiStatusExtended {
        installed: st.installed,
        running: st.running,
        models: st.models,
        recommended_model: rec.clone(),
        circuit_open: GLOBAL_AI_BREAKER.is_open(),
        model: rec,
        model_installed,
        task_running: false,
        task_percent: 0.0,
        task_message: String::new(),
        cache_count,
        matcher_mode: view.mode,
        cloud_provider: view.cloud_provider,
        cloud_model: view.cloud_model,
        cloud_base_url: view.cloud_base_url,
        cloud_has_key: view.cloud_has_key,
        cloud_key_masked: view.cloud_key_masked,
        ollama_model: view.ollama_model,
        active_engine_label,
    })
}

#[tauri::command]
pub async fn launch_ollama() -> Result<AiActionResp, JsfsError> {
    let exe = ollama::find_ollama_executable();
    let Some(path) = exe else {
        return Ok(AiActionResp {
            ok: false,
            progress: 0.0,
            message: "Aplikasi Ollama belum terpasang di komputer ini.".to_string(),
        });
    };

    tokio::task::spawn_blocking(move || -> Result<(), JsfsError> {
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
    .await
    .map_err(|e| JsfsError::Document(e.to_string()))??;

    Ok(AiActionResp {
        ok: true,
        progress: 100.0,
        message: "Layanan Ollama berhasil dijalankan.".to_string(),
    })
}

#[tauri::command]
pub async fn install_ollama(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AiActionResp, JsfsError> {
    let client = state.http_client.clone();
    tauri::async_runtime::spawn(async move {
        let _ = app.emit(
            "ai-status-changed",
            AiProgressEvent {
                running: false,
                model_installed: false,
                task_running: true,
                task_percent: 5.0,
                task_message: "Mengunduh OllamaSetup.exe...".to_string(),
            },
        );

        let url = "https://ollama.com/download/OllamaSetup.exe";
        let resp = match client.get(url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => {
                let _ = app.emit(
                    "ai-status-changed",
                    AiProgressEvent {
                        running: false,
                        model_installed: false,
                        task_running: false,
                        task_percent: 0.0,
                        task_message: "Gagal mengunduh OllamaSetup.exe.".to_string(),
                    },
                );
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
                let _ = app.emit(
                    "ai-status-changed",
                    AiProgressEvent {
                        running: false,
                        model_installed: false,
                        task_running: true,
                        task_percent: pct,
                        task_message: format!("Mengunduh Ollama ({:.0}%)...", pct),
                    },
                );
            }
        }

        let sp = setup_path.clone();
        let write_res = tokio::task::spawn_blocking(move || -> Result<(), JsfsError> {
            std::fs::write(&sp, &buf)?;
            std::process::Command::new(&sp).spawn()?;
            Ok(())
        })
        .await;

        let ok = matches!(write_res, Ok(Ok(())));
        let _ = app.emit(
            "ai-status-changed",
            AiProgressEvent {
                running: false,
                model_installed: false,
                task_running: false,
                task_percent: if ok { 100.0 } else { 0.0 },
                task_message: if ok {
                    "Installer Ollama dijalankan. Selesaikan wizard lalu klik Cek Ulang.".to_string()
                } else {
                    "Gagal menjalankan installer Ollama.".to_string()
                },
            },
        );
    });

    Ok(AiActionResp {
        ok: true,
        progress: 5.0,
        message: "Mengunduh installer Ollama di latar belakang...".to_string(),
    })
}

#[tauri::command]
pub async fn pull_ai_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model: Option<String>,
) -> Result<AiActionResp, JsfsError> {
    let target_model = model
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_AI_MODEL.to_string());
    let client = state.http_client.clone();
    let m_clone = target_model.clone();

    tauri::async_runtime::spawn(async move {
        let _ = app.emit(
            "ai-status-changed",
            AiProgressEvent {
                running: true,
                model_installed: false,
                task_running: true,
                task_percent: 2.0,
                task_message: format!("Mengunduh model {m_clone}..."),
            },
        );

        let body = serde_json::json!({ "name": m_clone, "stream": true });
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
                        let _ = app.emit(
                            "ai-status-changed",
                            AiProgressEvent {
                                running: true,
                                model_installed: status.contains("success"),
                                task_running: !status.contains("success"),
                                task_percent: pct,
                                task_message: format!("{m_clone}: {status}"),
                            },
                        );
                    }
                }
            }
        }

        let final_st = ollama::get_ai_status(&client).await;
        let base = m_clone.split(':').next().unwrap_or(&m_clone).to_ascii_lowercase();
        let installed = final_st
            .models
            .iter()
            .any(|m| m.to_ascii_lowercase().starts_with(&format!("{base}:")));
        let _ = app.emit(
            "ai-status-changed",
            AiProgressEvent {
                running: final_st.running,
                model_installed: installed,
                task_running: false,
                task_percent: if installed { 100.0 } else { 0.0 },
                task_message: if installed {
                    format!("Model {m_clone} siap digunakan.")
                } else {
                    format!("Unduhan {m_clone} selesai/dibatalkan.")
                },
            },
        );
    });

    Ok(AiActionResp {
        ok: true,
        progress: 2.0,
        message: format!("Mengunduh model {target_model}..."),
    })
}

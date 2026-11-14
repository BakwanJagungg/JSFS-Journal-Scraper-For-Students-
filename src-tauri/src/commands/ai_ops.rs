//! Ollama status, background launcher, one-click installer, and streaming model puller.

use crate::ai::ollama::{self, GLOBAL_AI_BREAKER};
use crate::config::{cache_dir, DEFAULT_AI_MODEL, OLLAMA_BASE_URL};
use crate::error::JsfsError;
use crate::AppState;
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

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

#[tauri::command]
pub async fn get_ai_status(state: State<'_, AppState>) -> Result<AiStatusExtended, JsfsError> {
    let st = ollama::get_ai_status(&state.http_client).await;
    let rec = DEFAULT_AI_MODEL.to_string();
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

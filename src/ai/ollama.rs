//! Optional local Ollama LLM integration using shared selective enrichment & breaker.

use super::{parse_json_lenient, run_selective_enrichment, GLOBAL_AI_BREAKER};
use crate::config::{DEFAULT_AI_MODEL, OLLAMA_BASE_URL, OLLAMA_PER_ITEM_TIMEOUT_SECS};
use crate::models::{AiStatus, JournalRecord};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

pub fn find_ollama_executable() -> Option<PathBuf> {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let cand = dir.join("ollama.exe");
            if cand.is_file() {
                return Some(cand);
            }
            let cand_nix = dir.join("ollama");
            if cand_nix.is_file() {
                return Some(cand_nix);
            }
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(local).join("Programs\\Ollama\\ollama.exe");
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(pf) = std::env::var("PROGRAMFILES") {
        let p = PathBuf::from(pf).join("Ollama\\ollama.exe");
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

pub async fn get_ai_status(client: &reqwest::Client) -> AiStatus {
    let exe_found = find_ollama_executable().is_some();
    let mut running = false;
    let mut models = Vec::new();

    if let Ok(resp) = client
        .get(format!("{OLLAMA_BASE_URL}/api/tags"))
        .timeout(Duration::from_millis(1500))
        .send()
        .await
    {
        if resp.status().is_success() {
            running = true;
            if let Ok(val) = resp.json::<Value>().await {
                if let Some(arr) = val.get("models").and_then(|v| v.as_array()) {
                    for m in arr {
                        if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                            models.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    AiStatus {
        installed: exe_found || running,
        running,
        models,
        recommended_model: DEFAULT_AI_MODEL.to_string(),
        circuit_open: GLOBAL_AI_BREAKER.is_open(),
        task_running: false,
        task_percent: 0.0,
        task_message: String::new(),
    }
}

async fn call_ollama_json(
    client: &reqwest::Client,
    model: &str,
    prompt: &str,
) -> Result<Value, String> {
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "format": "json",
        "keep_alive": "30m",
        "options": { "temperature": 0, "num_ctx": 4096, "num_predict": 256 }
    });
    let resp = client
        .post(format!("{OLLAMA_BASE_URL}/api/generate"))
        .json(&body)
        .timeout(Duration::from_secs(OLLAMA_PER_ITEM_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("http_{}", resp.status()));
    }
    let outer: Value = resp.json().await.map_err(|e| e.to_string())?;
    let resp_str = outer
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing response".to_string())?;
    parse_json_lenient(resp_str)
}

/// Enrich up to `OLLAMA_MAX_ITEMS` (8) highest-priority incomplete records via local Ollama.
pub async fn enrich_records_selective<F>(
    client: &reqwest::Client,
    records: &mut [JournalRecord],
    requested_model: Option<&str>,
    cancel_flag: Arc<AtomicBool>,
    on_progress: F,
) where
    F: Fn(usize, usize, &str) + Send + Sync,
{
    if GLOBAL_AI_BREAKER.is_open() {
        return;
    }
    let status = get_ai_status(client).await;
    if !status.running || status.models.is_empty() {
        return;
    }

    let chosen_model = requested_model
        .filter(|m| !m.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            status
                .models
                .iter()
                .find(|m| m.starts_with(DEFAULT_AI_MODEL))
                .cloned()
        })
        .unwrap_or_else(|| status.models[0].clone());

    let src = format!("Ollama:{chosen_model}");
    let model_ref = chosen_model.as_str();
    run_selective_enrichment(records, &src, cancel_flag, on_progress, |prompt| async move {
        call_ollama_json(client, model_ref, &prompt).await
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::super::validate_apc_json;

    #[test]
    fn test_ai_apc_hallucination_and_submission_guard() {
        let snippet = "Author Fees: Article Submission: 0 (IDR). Publication charge not listed.";
        let hallucinated = serde_json::json!({
            "biaya_submit": 0,
            "biaya_publikasi": 0,
            "mata_uang": "IDR"
        });
        assert!(validate_apc_json(&hallucinated, snippet).is_none());

        let valid_pub = serde_json::json!({
            "biaya_submit": 0,
            "biaya_publikasi": 750000,
            "mata_uang": "IDR"
        });
        assert!(
            validate_apc_json(&valid_pub, "Submission 0. Publication fee: Rp 750.000").is_some()
        );
        assert!(validate_apc_json(&valid_pub, "Submission 0. Publication fee: Rp 500.000").is_none());
    }
}

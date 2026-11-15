//! Optional free Cloud AI API provider (Groq, Google Gemini, or OpenAI-Compatible).

use super::{parse_json_lenient, run_selective_enrichment, GLOBAL_AI_BREAKER};
use crate::config::{MatcherConfig, CLOUD_PER_ITEM_TIMEOUT_SECS};
use crate::models::JournalRecord;
use serde_json::Value;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

const GROQ_CHAT_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Resolve endpoint URL for OpenAI-compatible providers (`groq` or custom `base_url`).
pub fn resolve_openai_chat_url(provider: &str, base_url: &str) -> String {
    if provider == "groq" {
        return GROQ_CHAT_URL.to_string();
    }
    let clean = base_url.trim().trim_end_matches('/');
    if clean.is_empty() {
        "https://api.openai.com/v1/chat/completions".to_string()
    } else if clean.ends_with("/chat/completions") {
        clean.to_string()
    } else {
        format!("{clean}/chat/completions")
    }
}

/// Resolve Google Gemini `generateContent` endpoint for the chosen model.
pub fn resolve_gemini_url(model: &str, api_key: &str) -> String {
    let m = model.trim().trim_start_matches("models/");
    let encoded_key = urlencoding::encode(api_key.trim());
    format!("{GEMINI_API_BASE}/{m}:generateContent?key={encoded_key}")
}

async fn call_cloud_json(
    client: &reqwest::Client,
    cfg: &MatcherConfig,
    prompt: &str,
) -> Result<Value, String> {
    let key = cfg.cloud_api_key.trim();
    if key.is_empty() {
        return Err("API Key belum diisi".to_string());
    }
    let provider = cfg.normalized_provider();
    let model = cfg.effective_cloud_model();

    if provider == "gemini" {
        let url = resolve_gemini_url(&model, key);
        let body = serde_json::json!({
            "contents": [{ "parts": [{ "text": prompt }] }],
            "generationConfig": {
                "temperature": 0,
                "maxOutputTokens": 256,
                "responseMimeType": "application/json"
            }
        });
        let resp = client
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(CLOUD_PER_ITEM_TIMEOUT_SECS))
            .send()
            .await
            .map_err(|e| format!("Koneksi Gemini gagal: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_txt = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Gemini HTTP {status}: {}",
                err_txt.chars().take(160).collect::<String>()
            ));
        }
        let val: Value = resp.json().await.map_err(|e| e.to_string())?;
        let text = val["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| "Respons Gemini kosong".to_string())?;
        return parse_json_lenient(text);
    }

    // Groq & OpenAI-Compatible chat completions
    let url = resolve_openai_chat_url(provider, &cfg.cloud_base_url);
    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": "You return only valid minified JSON without markdown." },
            { "role": "user", "content": prompt }
        ],
        "temperature": 0,
        "max_tokens": 256
    });
    if provider == "groq" {
        body["response_format"] = serde_json::json!({ "type": "json_object" });
    }

    let resp = client
        .post(&url)
        .bearer_auth(key)
        .json(&body)
        .timeout(Duration::from_secs(CLOUD_PER_ITEM_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| format!("Koneksi Cloud API gagal: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_txt = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Cloud HTTP {status}: {}",
            err_txt.chars().take(160).collect::<String>()
        ));
    }

    let val: Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = val["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "Respons Cloud API kosong".to_string())?;
    parse_json_lenient(content)
}

/// Live connection test for Groq / Gemini / OpenAI-compatible settings.
pub async fn test_cloud_connection(
    client: &reqwest::Client,
    cfg: &MatcherConfig,
) -> Result<String, String> {
    let started = Instant::now();
    let ping_prompt = r#"Balas HANYA JSON persis berikut: {"ok": true}"#;
    let val = call_cloud_json(client, cfg, ping_prompt).await?;
    let ms = started.elapsed().as_millis();
    let ok = val.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) || val.is_object();
    if ok {
        Ok(format!(
            "Koneksi {} ({}) berhasil ({} ms).",
            cfg.normalized_provider().to_uppercase(),
            cfg.effective_cloud_model(),
            ms
        ))
    } else {
        Err("Respons JSON tidak valid.".to_string())
    }
}

/// Enrich up to `OLLAMA_MAX_ITEMS` (8) highest-priority incomplete records via Cloud AI API.
pub async fn enrich_records_cloud<F>(
    client: &reqwest::Client,
    records: &mut [JournalRecord],
    cfg: &MatcherConfig,
    cancel_flag: Arc<AtomicBool>,
    on_progress: F,
) where
    F: Fn(usize, usize, &str) + Send + Sync,
{
    if GLOBAL_AI_BREAKER.is_open() || cfg.cloud_api_key.trim().is_empty() {
        return;
    }
    let src = format!(
        "Cloud:{}/{}",
        cfg.normalized_provider(),
        cfg.effective_cloud_model()
    );
    run_selective_enrichment(
        records,
        &src,
        cancel_flag,
        on_progress,
        |prompt| async move { call_cloud_json(client, cfg, &prompt).await },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_endpoint_resolution_and_lenient_json() {
        assert_eq!(resolve_openai_chat_url("groq", ""), GROQ_CHAT_URL);
        assert_eq!(
            resolve_openai_chat_url("openai_compatible", "https://openrouter.ai/api/v1/"),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            resolve_openai_chat_url(
                "openai_compatible",
                "https://api.example.com/v1/chat/completions"
            ),
            "https://api.example.com/v1/chat/completions"
        );
        let gemini = resolve_gemini_url("gemini-2.5-flash", "AIzaTestKey");
        assert!(gemini.contains("gemini-2.5-flash:generateContent?key=AIzaTestKey"));

        let fenced =
            "```json\n{\"focus_scope\": \"Riset sistem informasi dan bisnis digital.\"}\n```";
        let parsed = parse_json_lenient(fenced).unwrap();
        assert_eq!(
            parsed["focus_scope"].as_str(),
            Some("Riset sistem informasi dan bisnis digital.")
        );
    }
}

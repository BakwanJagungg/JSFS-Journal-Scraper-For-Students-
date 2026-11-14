//! Manuscript upload & parsing command (`.docx` / `.pdf` via spawn_blocking).

use crate::error::JsfsError;
use crate::parser::doc_matcher;
use crate::AppState;
use base64::Engine;
use serde::Serialize;
use std::path::Path;
use tauri::State;

#[derive(Serialize)]
pub struct ManuscriptResp {
    pub ok: bool,
    pub path: String,
    pub name: String,
    pub suggested_keyword: String,
    pub char_count: usize,
    pub detected_domain: String,
    pub detected_keywords: Vec<String>,
    pub message: String,
}

#[tauri::command]
pub async fn upload_manuscript(
    state: State<'_, AppState>,
    path: Option<String>,
    filename: Option<String>,
    bytes_b64: Option<String>,
    clear: Option<bool>,
) -> Result<ManuscriptResp, JsfsError> {
    if clear.unwrap_or(false) {
        let mut guard = state.manuscript.write().await;
        *guard = None;
        return Ok(ManuscriptResp {
            ok: true,
            path: String::new(),
            name: String::new(),
            suggested_keyword: String::new(),
            char_count: 0,
            detected_domain: String::new(),
            detected_keywords: vec![],
            message: "Lampiran naskah dibersihkan.".to_string(),
        });
    }

    let meta = if let Some(b64) = bytes_b64.filter(|s| !s.trim().is_empty()) {
        let fname = filename
            .unwrap_or_else(|| "naskah.docx".to_string())
            .trim()
            .to_string();
        let raw = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|e| JsfsError::Document(format!("Base64 naskah tidak valid: {e}")))?;
        let fname_clone = fname.clone();
        tokio::task::spawn_blocking(move || doc_matcher::build_manuscript_meta(&fname_clone, &raw))
            .await
            .map_err(|e| JsfsError::Document(e.to_string()))??
    } else if let Some(p) = path.filter(|s| !s.trim().is_empty()) {
        let path_obj = Path::new(&p);
        let fname = path_obj
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("naskah.docx")
            .to_string();
        let p_clone = p.clone();
        tokio::task::spawn_blocking(move || -> Result<_, JsfsError> {
            let raw = std::fs::read(&p_clone)?;
            doc_matcher::build_manuscript_meta(&fname, &raw)
        })
        .await
        .map_err(|e| JsfsError::Document(e.to_string()))??
    } else {
        return Err(JsfsError::InvalidInput(
            "Berkas naskah (.docx atau .pdf) tidak disertakan.".to_string(),
        ));
    };

    let suggested_keyword = meta
        .detected_keywords
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let name = meta.filename.clone();
    let char_count = meta.char_count;
    let detected_domain = meta.detected_domain.clone();
    let detected_keywords = meta.detected_keywords.clone();

    {
        let mut guard = state.manuscript.write().await;
        *guard = Some(meta);
    }

    Ok(ManuscriptResp {
        ok: true,
        path: name.clone(),
        name,
        suggested_keyword,
        char_count,
        detected_domain,
        detected_keywords,
        message: "Naskah berhasil diindeks.".to_string(),
    })
}

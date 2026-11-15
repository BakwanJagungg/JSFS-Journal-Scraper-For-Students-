//! Native manuscript file loader & analyzer (`.docx` / `.pdf` via `spawn_blocking`).

use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::error::JsfsError;
use crate::parser::doc_matcher;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct ManuscriptSummary {
    pub path: String,
    pub name: String,
    pub extracted_title: String,
    pub suggested_keyword: String,
    pub abstract_snippet: String,
    pub char_count: usize,
    pub detected_domain: String,
    pub detected_keywords: Vec<String>,
    pub word_count: usize,
    pub imrad_complete: bool,
    pub estimated_refs: usize,
    pub summary_badge: String,
    pub cited_journals: HashMap<String, u32>,
    pub recommended_topics: Vec<String>,
}

pub fn load_manuscript_async(engine: Arc<EngineState>, sink: EventSink, path: PathBuf) {
    let rt = engine.runtime.handle().clone();
    rt.spawn(async move {
        let fname = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("naskah.docx")
            .to_string();
        let display_path = path.to_string_lossy().into_owned();
        let path_clone = path.clone();

        let parsed = tokio::task::spawn_blocking(move || -> Result<_, JsfsError> {
            let ext = path_clone
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "docm" {
                return Err(JsfsError::InvalidInput(
                    "File .docm (macro-enabled) ditolak demi keamanan; gunakan .docx atau .pdf standar."
                        .to_string(),
                ));
            }
            if ext != "docx" && ext != "pdf" {
                return Err(JsfsError::InvalidInput(
                    "Hanya file .docx atau .pdf yang didukung".to_string(),
                ));
            }
            let meta_fs = std::fs::metadata(&path_clone)?;
            if meta_fs.len() > crate::parser::doc_matcher::MAX_MANUSCRIPT_BYTES as u64 {
                return Err(JsfsError::InvalidInput(
                    "Ukuran file naskah melebihi batas maksimal 50 MB.".to_string(),
                ));
            }
            let raw = std::fs::read(&path_clone)?;
            doc_matcher::build_manuscript_meta(&fname, &raw)
        })
        .await;

        match parsed {
            Ok(Ok(meta)) => {
                let suggested_keyword = if !meta.suggested_keywords.trim().is_empty() {
                    meta.suggested_keywords.clone()
                } else {
                    meta.detected_keywords
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let summary = ManuscriptSummary {
                    path: display_path,
                    name: meta.filename.clone(),
                    extracted_title: meta.extracted_title.clone(),
                    suggested_keyword,
                    abstract_snippet: meta.abstract_snippet.clone(),
                    char_count: meta.char_count,
                    detected_domain: meta.detected_domain.clone(),
                    detected_keywords: meta.detected_keywords.clone(),
                    word_count: meta.checklist.word_count,
                    imrad_complete: meta.checklist.imrad_complete,
                    estimated_refs: meta.checklist.estimated_refs,
                    summary_badge: meta.checklist.summary_badge.clone(),
                    cited_journals: meta.cited_journals.clone(),
                    recommended_topics: meta.recommended_topics.clone(),
                };
                {
                    let mut guard = engine.manuscript.write().await;
                    *guard = Some(meta);
                }
                sink.send(EngineEvent::ManuscriptLoaded(Ok(summary)));
            }
            Ok(Err(e)) => {
                sink.send(EngineEvent::ManuscriptLoaded(Err(e.to_string())));
            }
            Err(e) => {
                sink.send(EngineEvent::ManuscriptLoaded(Err(e.to_string())));
            }
        }
    });
}

pub fn clear_manuscript_async(engine: Arc<EngineState>, sink: EventSink) {
    let rt = engine.runtime.handle().clone();
    rt.spawn(async move {
        let mut guard = engine.manuscript.write().await;
        *guard = None;
        sink.send(EngineEvent::Toast(
            "Lampiran naskah dibersihkan.".to_string(),
        ));
    });
}

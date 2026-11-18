//! Native manuscript file loader & analyzer (`.docx` / `.pdf` via `spawn_blocking`).

use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::error::JsfsError;
use crate::models::ManuscriptMeta;
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
    pub uncited_long_paragraphs: u32,
}

/// Parse and validate manuscript bytes (.docx / .pdf), returning a user-friendly error if PDF lacks text (e.g. scanned image).
pub fn parse_manuscript(filename: &str, raw: &[u8]) -> Result<ManuscriptMeta, JsfsError> {
    let ext = std::path::Path::new(filename)
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
    if raw.len() > crate::parser::doc_matcher::MAX_MANUSCRIPT_BYTES {
        return Err(JsfsError::InvalidInput(
            "Ukuran file naskah melebihi batas maksimal 50 MB.".to_string(),
        ));
    }
    let meta = doc_matcher::build_manuscript_meta(filename, raw)?;
    if ext == "pdf" && (meta.char_count < 30 || meta.checklist.word_count == 0) {
        return Err(JsfsError::Document(
            "Berkas PDF tidak memiliki lapisan teks (kemungkinan hasil scan gambar). Silakan unggah berkas .docx asli dari Microsoft Word."
                .to_string(),
        ));
    }
    Ok(meta)
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
            let meta_fs = std::fs::metadata(&path_clone)?;
            if meta_fs.len() > crate::parser::doc_matcher::MAX_MANUSCRIPT_BYTES as u64 {
                return Err(JsfsError::InvalidInput(
                    "Ukuran file naskah melebihi batas maksimal 50 MB.".to_string(),
                ));
            }
            let raw = std::fs::read(&path_clone)?;
            parse_manuscript(&fname, &raw)
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
                    uncited_long_paragraphs: meta.checklist.uncited_long_paragraphs,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manuscript_empty_or_scanned_pdf_rejection() {
        // Valid minimal 1-page blank PDF bytes without text layer
        const EMPTY_PDF: &[u8] = &[
            37, 80, 68, 70, 45, 49, 46, 52, 10, 49, 32, 48, 32, 111, 98, 106, 10, 60, 60, 32, 47,
            84, 121, 112, 101, 32, 47, 67, 97, 116, 97, 108, 111, 103, 32, 47, 80, 97, 103, 101,
            115, 32, 50, 32, 48, 32, 82, 32, 62, 62, 10, 101, 110, 100, 111, 98, 106, 10, 50, 32,
            48, 32, 111, 98, 106, 10, 60, 60, 32, 47, 84, 121, 112, 101, 32, 47, 80, 97, 103,
            101, 115, 32, 47, 75, 105, 100, 115, 32, 91, 51, 32, 48, 32, 82, 93, 32, 47, 67, 111,
            117, 110, 116, 32, 49, 32, 62, 62, 10, 101, 110, 100, 111, 98, 106, 10, 51, 32, 48,
            32, 111, 98, 106, 10, 60, 60, 32, 47, 84, 121, 112, 101, 32, 47, 80, 97, 103, 101,
            32, 47, 80, 97, 114, 101, 110, 116, 32, 50, 32, 48, 32, 82, 32, 47, 77, 101, 100,
            105, 97, 66, 111, 120, 32, 91, 48, 32, 48, 32, 54, 49, 50, 32, 55, 57, 50, 93, 32,
            62, 62, 10, 101, 110, 100, 111, 98, 106, 10, 120, 114, 101, 102, 10, 48, 32, 52, 10,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 32, 54, 53, 53, 51, 53, 32, 102, 32, 10, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 57, 32, 48, 48, 48, 48, 48, 32, 110, 32, 10, 48, 48,
            48, 48, 48, 48, 48, 48, 53, 56, 32, 48, 48, 48, 48, 48, 32, 110, 32, 10, 48, 48, 48,
            48, 48, 48, 48, 49, 49, 53, 32, 48, 48, 48, 48, 48, 32, 110, 32, 10, 116, 114, 97,
            105, 108, 101, 114, 10, 60, 60, 32, 47, 83, 105, 122, 101, 32, 52, 32, 47, 82, 111,
            111, 116, 32, 49, 32, 48, 32, 82, 32, 62, 62, 10, 115, 116, 97, 114, 116, 120, 114,
            101, 102, 10, 49, 56, 54, 10, 37, 37, 69, 79, 70, 10,
        ];

        let result = parse_manuscript("skripsi_scan.pdf", EMPTY_PDF);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg
                .contains("Berkas PDF tidak memiliki lapisan teks (kemungkinan hasil scan gambar)"),
            "Expected friendly scan error, got: {err_msg}"
        );
    }
}

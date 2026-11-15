//! Native manuscript file loader & analyzer (`.docx` / `.pdf` via `spawn_blocking`).

use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::error::JsfsError;
use crate::parser::doc_matcher;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct ManuscriptSummary {
    pub path: String,
    pub name: String,
    pub suggested_keyword: String,
    pub char_count: usize,
    pub detected_domain: String,
    pub detected_keywords: Vec<String>,
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
            let raw = std::fs::read(&path_clone)?;
            doc_matcher::build_manuscript_meta(&fname, &raw)
        })
        .await;

        match parsed {
            Ok(Ok(meta)) => {
                let suggested_keyword = meta
                    .detected_keywords
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                let summary = ManuscriptSummary {
                    path: display_path,
                    name: meta.filename.clone(),
                    suggested_keyword,
                    char_count: meta.char_count,
                    detected_domain: meta.detected_domain.clone(),
                    detected_keywords: meta.detected_keywords.clone(),
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

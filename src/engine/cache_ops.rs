//! Cache clearing helper offloaded cleanly to `spawn_blocking`.

use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::error::JsfsError;
use std::sync::Arc;

pub fn clear_cache_async(engine: Arc<EngineState>, sink: EventSink) {
    let cache = engine.cache.clone();
    engine.runtime.spawn(async move {
        let res = tokio::task::spawn_blocking(move || -> Result<(usize, usize), JsfsError> {
            let guard = cache
                .lock()
                .map_err(|e| JsfsError::Document(format!("Lock error: {e}")))?;
            if let Some(ref db) = *guard {
                let n = db.clear()?;
                db.checkpoint_wal();
                let remaining = db.count_valid().unwrap_or(0);
                Ok((n, remaining))
            } else {
                Ok((0, 0))
            }
        })
        .await;

        match res {
            Ok(Ok((cleared, remaining))) => {
                sink.send(EngineEvent::CacheCleared(Ok((cleared, remaining))));
                sink.send(EngineEvent::Toast(format!(
                    "Cache SQLite dibersihkan ({cleared} entri dihapus)."
                )));
            }
            Ok(Err(e)) => sink.send(EngineEvent::CacheCleared(Err(e.to_string()))),
            Err(e) => sink.send(EngineEvent::CacheCleared(Err(e.to_string()))),
        }
    });
}

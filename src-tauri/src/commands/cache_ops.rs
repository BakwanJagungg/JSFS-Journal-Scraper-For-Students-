//! Cache clearing command executed cleanly in spawn_blocking.

use crate::error::JsfsError;
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ClearCacheResp {
    pub ok: bool,
    pub cleared: usize,
    pub cache_count: usize,
    pub message: String,
}

#[tauri::command]
pub async fn clear_cache(state: State<'_, AppState>) -> Result<ClearCacheResp, JsfsError> {
    let cache = state.cache.clone();
    let cleared = tokio::task::spawn_blocking(move || -> Result<usize, JsfsError> {
        let guard = cache
            .lock()
            .map_err(|e| JsfsError::Document(format!("Lock error: {e}")))?;
        if let Some(ref db) = *guard {
            let n = db.clear()?;
            db.checkpoint_wal();
            Ok(n)
        } else {
            Ok(0)
        }
    })
    .await
    .map_err(|e| JsfsError::Document(e.to_string()))??;

    Ok(ClearCacheResp {
        ok: true,
        cleared,
        cache_count: 0,
        message: format!("Cache lokal SQLite berhasil dibersihkan ({cleared} entri dihapus)."),
    })
}

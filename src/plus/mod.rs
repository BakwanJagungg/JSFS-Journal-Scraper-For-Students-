//! Isolated JSFS+ Workstation capabilities:
//! - Headless Chromium/Edge Sidecar for Cloudflare & JS-rendered OJS portals
//! - Scanned PDF OCR recovery bridge (Tesseract CLI / Windows WinRT OCR)
//! - Hybrid Semantic Reranker (Pure Rust Vector Projection + optional Ollama Embeddings + Citation Velocity)

pub mod browser_sidecar;
pub mod ocr;
pub mod semantic;

use std::path::PathBuf;

/// Runtime detection summary shown in JSFS+ Workstation Settings & diagnostics.
#[derive(Debug, Clone)]
pub struct PlusCapabilities {
    pub headless_browser_path: Option<PathBuf>,
    pub ocr_engine_label: Option<String>,
    pub max_target_count: usize,
    pub max_workers: usize,
}

pub fn detect_capabilities() -> PlusCapabilities {
    PlusCapabilities {
        headless_browser_path: browser_sidecar::detect_headless_browser(),
        ocr_engine_label: ocr::detect_available_ocr_label(),
        max_target_count: crate::config::MAX_TARGET_COUNT,
        max_workers: crate::config::effective_max_workers(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plus_scale_limits_configured() {
        assert_eq!(crate::config::MAX_TARGET_COUNT, 5000);
        assert_eq!(crate::config::SQLITE_CACHE_SIZE_KIB, 131_072);
        assert_eq!(crate::config::MAX_HTML_BYTES, 12 * 1024 * 1024);
        const { assert!(crate::config::IS_PLUS_EDITION) };
        assert_eq!(crate::config::APP_VERSION, "5.0.0-plus");
        let caps = detect_capabilities();
        assert_eq!(caps.max_target_count, 5000);
        assert!(caps.max_workers >= 24);
    }
}

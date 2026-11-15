//! OS file/folder/browser launchers and copyable WhatsApp/editor draft generators.

use crate::config::{logs_dir, output_dir};
use crate::engine::{EngineEvent, EngineState, EventSink};
use crate::error::JsfsError;
use crate::exporter::list_txt::generate_copypaste_list;
use crate::models::{JournalRecord, UiJournalItem};
use std::sync::Arc;

pub fn open_with_os(target: &str) -> Result<(), JsfsError> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", target])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(target).spawn()?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(target).spawn()?;
    }
    Ok(())
}

pub fn open_url_with_feedback(url: &str, sink: &EventSink) {
    let clean = url.trim();
    if !clean.starts_with("http://") && !clean.starts_with("https://") {
        sink.send(EngineEvent::Toast(
            "URL harus diawali http:// atau https://".to_string(),
        ));
        return;
    }
    match open_with_os(clean) {
        Ok(()) => sink.send(EngineEvent::Toast(
            "Membuka tautan di browser...".to_string(),
        )),
        Err(e) => sink.send(EngineEvent::Toast(format!("Gagal membuka URL: {e}"))),
    }
}

pub fn open_excel_async(engine: Arc<EngineState>, sink: EventSink, explicit_path: Option<String>) {
    let rt = engine.runtime.handle().clone();
    rt.spawn(async move {
        let resolved = if let Some(p) = explicit_path.filter(|s| !s.trim().is_empty()) {
            p
        } else {
            engine
                .last_excel_path
                .read()
                .await
                .clone()
                .unwrap_or_default()
        };

        if resolved.is_empty() || !std::path::Path::new(&resolved).exists() {
            sink.send(EngineEvent::Toast(
                "File Excel hasil pencarian belum tersedia.".to_string(),
            ));
            return;
        }

        let res = tokio::task::spawn_blocking(move || open_with_os(&resolved)).await;
        match res {
            Ok(Ok(())) => sink.send(EngineEvent::Toast("Membuka file Excel...".to_string())),
            _ => sink.send(EngineEvent::Toast(
                "Gagal membuka file Excel dengan aplikasi bawaan.".to_string(),
            )),
        }
    });
}

pub fn open_dir_async(
    engine: &EngineState,
    sink: EventSink,
    dir: std::path::PathBuf,
    label: &'static str,
) {
    engine.runtime.spawn(async move {
        let res = tokio::task::spawn_blocking(move || -> Result<(), JsfsError> {
            std::fs::create_dir_all(&dir)?;
            open_with_os(&dir.to_string_lossy())
        })
        .await;
        match res {
            Ok(Ok(())) => sink.send(EngineEvent::Toast(format!("Membuka folder {label}..."))),
            _ => sink.send(EngineEvent::Toast(format!("Gagal membuka folder {label}."))),
        }
    });
}

pub fn open_output_folder_async(engine: &EngineState, sink: EventSink) {
    open_dir_async(engine, sink, output_dir(), "output");
}

pub fn open_logs_folder_async(engine: &EngineState, sink: EventSink) {
    open_dir_async(engine, sink, logs_dir(), "logs");
}

/// Formats an instant WhatsApp share block from visible UI rows without blocking UI rendering.
pub fn format_wa_share_from_ui(items: &[&UiJournalItem]) -> String {
    let recs: Vec<JournalRecord> = items
        .iter()
        .map(|it| JournalRecord {
            title: it.nama.clone(),
            sinta_score: if it.sinta.is_empty() {
                String::new()
            } else {
                format!("Sinta {}", it.sinta)
            },
            apc_status: it.apc_label.clone(),
            publish_schedule: it.jadwal_terbit.clone(),
            next_issue_estimate: it.terbit_terdekat.clone(),
            email: it.email.clone(),
            phone: it.telepon.clone(),
            website_url: it.url.clone(),
            garuda_url: it.garuda_url.clone(),
            ..Default::default()
        })
        .collect();
    generate_copypaste_list(&recs)
}

/// Formats a polite Indonesian inquiry message addressed to the journal editor.
pub fn format_editor_draft(journal_title: &str, keyword: &str) -> String {
    let title = journal_title.trim();
    let kw_text = if keyword.trim().is_empty() {
        "bidang penelitian saya"
    } else {
        keyword.trim()
    };
    format!(
        "Selamat pagi/siang Bapak/Ibu Tim Editor {title},\n\n\
Perkenalkan saya mahasiswa yang sedang menyusun artikel ilmiah di bidang {kw_text}. \
Melalui pesan ini saya ingin menanyakan apakah masih tersedia kuota/slot publikasi \
untuk edisi terbit terdekat serta konfirmasi mengenai estimasi biaya publikasi (APC) \
di {title}.\n\n\
Terima kasih banyak atas perhatian dan informasinya."
    )
}

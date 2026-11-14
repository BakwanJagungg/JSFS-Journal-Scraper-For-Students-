//! OS file/folder/browser launchers and copyable text draft commands.

use crate::config::{logs_dir, output_dir};
use crate::error::JsfsError;
use crate::exporter::list_txt::generate_copypaste_list;
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct FileOpResp {
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct CopyTextResp {
    pub ok: bool,
    pub text: String,
}

fn open_with_os(target: &str) -> Result<(), JsfsError> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", target])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open").arg(target).spawn()?;
    }
    Ok(())
}

#[tauri::command]
pub async fn open_excel(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<FileOpResp, JsfsError> {
    let resolved = if let Some(p) = path.filter(|s| !s.trim().is_empty()) {
        p
    } else {
        state
            .last_excel_path
            .read()
            .await
            .clone()
            .unwrap_or_default()
    };

    if resolved.is_empty() || !std::path::Path::new(&resolved).exists() {
        return Ok(FileOpResp {
            ok: false,
            message: "File Excel hasil pencarian belum tersedia.".to_string(),
        });
    }

    tokio::task::spawn_blocking(move || open_with_os(&resolved))
        .await
        .map_err(|e| JsfsError::Document(e.to_string()))??;
    Ok(FileOpResp {
        ok: true,
        message: "Membuka file Excel...".to_string(),
    })
}

#[tauri::command]
pub async fn open_output_folder() -> Result<FileOpResp, JsfsError> {
    let dir = output_dir();
    tokio::task::spawn_blocking(move || -> Result<(), JsfsError> {
        std::fs::create_dir_all(&dir)?;
        open_with_os(&dir.to_string_lossy())
    })
    .await
    .map_err(|e| JsfsError::Document(e.to_string()))??;

    Ok(FileOpResp {
        ok: true,
        message: "Membuka folder output...".to_string(),
    })
}

#[tauri::command]
pub async fn open_logs_folder() -> Result<FileOpResp, JsfsError> {
    let dir = logs_dir();
    tokio::task::spawn_blocking(move || -> Result<(), JsfsError> {
        std::fs::create_dir_all(&dir)?;
        open_with_os(&dir.to_string_lossy())
    })
    .await
    .map_err(|e| JsfsError::Document(e.to_string()))??;

    Ok(FileOpResp {
        ok: true,
        message: "Membuka folder logs...".to_string(),
    })
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<FileOpResp, JsfsError> {
    let clean = url.trim().to_string();
    if !clean.starts_with("http://") && !clean.starts_with("https://") {
        return Ok(FileOpResp {
            ok: false,
            message: "URL harus diawali http:// atau https://".to_string(),
        });
    }

    tokio::task::spawn_blocking(move || open_with_os(&clean))
        .await
        .map_err(|e| JsfsError::Document(e.to_string()))??;

    Ok(FileOpResp {
        ok: true,
        message: "Membuka tautan di browser...".to_string(),
    })
}

#[tauri::command]
pub async fn copy_share_list(
    state: State<'_, AppState>,
    indices: Option<Vec<usize>>,
) -> Result<CopyTextResp, JsfsError> {
    let results = state.current_results.read().await;
    if results.is_empty() {
        return Ok(CopyTextResp {
            ok: false,
            text: String::new(),
        });
    }

    let subset: Vec<_> = if let Some(idxs) = indices.filter(|v| !v.is_empty()) {
        idxs.into_iter()
            .filter_map(|i| results.get(i).cloned())
            .collect()
    } else {
        results.clone()
    };
    let final_list = if subset.is_empty() {
        results.clone()
    } else {
        subset
    };
    let text = generate_copypaste_list(&final_list);
    Ok(CopyTextResp { ok: true, text })
}

#[tauri::command]
pub async fn copy_editor_draft(
    state: State<'_, AppState>,
    index: usize,
    keyword: Option<String>,
) -> Result<CopyTextResp, JsfsError> {
    let results = state.current_results.read().await;
    let Some(journal) = results.get(index) else {
        return Ok(CopyTextResp {
            ok: false,
            text: String::new(),
        });
    };

    let kw_text = keyword
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "bidang penelitian saya".to_string());

    let text = format!(
        "Selamat pagi/siang Bapak/Ibu Tim Editor {},\n\n\
        Perkenalkan saya mahasiswa yang sedang menyusun artikel ilmiah di bidang {kw_text}. \
        Melalui pesan ini saya ingin menanyakan apakah masih tersedia kuota/slot publikasi \
        untuk edisi terbit terdekat serta konfirmasi mengenai estimasi biaya publikasi (APC) \
        di {}.\n\n\
        Terima kasih banyak atas perhatian dan informasinya.",
        journal.title.trim(),
        journal.title.trim()
    );

    Ok(CopyTextResp { ok: true, text })
}

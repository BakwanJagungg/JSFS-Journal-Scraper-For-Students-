//! Local Scanned-PDF OCR Recovery Bridge for JSFS+ Workstation.
//! Automatically triggers when `pdf-extract` yields < 80 characters on a `.pdf` file,
//! utilizing system `tesseract` CLI or Windows native WinRT OCR when available.

use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct OcrRecoveryResult {
    pub text: String,
    pub recovered_via_ocr: bool,
    pub engine_label: Option<String>,
    pub status_note: Option<String>,
}

fn find_in_path(bins: &[&str]) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        for bin in bins {
            let p = dir.join(bin);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Detect available local OCR capability without spawning subprocesses.
pub fn detect_available_ocr_label() -> Option<String> {
    #[cfg(target_os = "windows")]
    let tess_bins = &["tesseract.exe"];
    #[cfg(not(target_os = "windows"))]
    let tess_bins = &["tesseract"];

    if find_in_path(tess_bins).is_some() {
        return Some("Tesseract OCR (PATH)".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        Some("Windows Native WinRT OCR".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Attempt local OCR recovery when `extracted_text` has fewer than 80 characters.
/// Degrades gracefully without failing if the system lacks external OCR tooling.
pub fn recover_pdf_text_with_ocr_fallback(
    pdf_bytes: &[u8],
    extracted_text: &str,
) -> OcrRecoveryResult {
    let trimmed = extracted_text.trim();
    if trimmed.chars().count() >= 80 {
        return OcrRecoveryResult {
            text: trimmed.to_string(),
            recovered_via_ocr: false,
            engine_label: None,
            status_note: None,
        };
    }

    if pdf_bytes.is_empty() {
        return OcrRecoveryResult::default();
    }

    let work_dir = std::env::temp_dir().join(format!("jsfs-plus-ocr-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work_dir);
    if std::fs::create_dir_all(&work_dir).is_err() {
        return OcrRecoveryResult {
            text: trimmed.to_string(),
            recovered_via_ocr: false,
            engine_label: None,
            status_note: Some("Folder sementara OCR tidak dapat dibuat.".to_string()),
        };
    }
    let pdf_path = work_dir.join("scan_input.pdf");
    if std::fs::write(&pdf_path, pdf_bytes).is_err() {
        let _ = std::fs::remove_dir_all(&work_dir);
        return OcrRecoveryResult {
            text: trimmed.to_string(),
            recovered_via_ocr: false,
            engine_label: None,
            status_note: Some("Gagal menulis PDF sementara untuk OCR.".to_string()),
        };
    }

    // 1. Try pdftoppm + tesseract if installed
    if let Some((txt, label)) = try_tesseract_pipeline(&pdf_path, &work_dir) {
        let _ = std::fs::remove_dir_all(&work_dir);
        if txt.trim().chars().count() >= 40 {
            return OcrRecoveryResult {
                text: txt.trim().to_string(),
                recovered_via_ocr: true,
                engine_label: Some(label),
                status_note: Some(
                    "Teks PDF scan berhasil dipulihkan melalui OCR lokal.".to_string(),
                ),
            };
        }
    }

    // 2. On Windows 10/11, try isolated WinRT Windows.Data.Pdf + Windows.Media.Ocr bridge
    #[cfg(target_os = "windows")]
    if let Some((txt, label)) = try_windows_winrt_ocr(&pdf_path) {
        let _ = std::fs::remove_dir_all(&work_dir);
        if txt.trim().chars().count() >= 40 {
            return OcrRecoveryResult {
                text: txt.trim().to_string(),
                recovered_via_ocr: true,
                engine_label: Some(label),
                status_note: Some(
                    "Teks PDF scan berhasil dipulihkan melalui Windows Native WinRT OCR."
                        .to_string(),
                ),
            };
        }
    }

    let _ = std::fs::remove_dir_all(&work_dir);
    OcrRecoveryResult {
        text: trimmed.to_string(),
        recovered_via_ocr: false,
        engine_label: None,
        status_note: Some(
            "PDF hasil scan tidak menghasilkan teks OCR yang memadai (pasang Tesseract OCR atau gunakan PDF/DOCX berteks)."
                .to_string(),
        ),
    }
}

fn try_tesseract_pipeline(pdf_path: &PathBuf, work_dir: &PathBuf) -> Option<(String, String)> {
    #[cfg(target_os = "windows")]
    let (tess_bins, ppm_bins) = (&["tesseract.exe"][..], &["pdftoppm.exe"][..]);
    #[cfg(not(target_os = "windows"))]
    let (tess_bins, ppm_bins) = (&["tesseract"][..], &["pdftoppm"][..]);

    let tess = find_in_path(tess_bins)?;
    let pdftoppm = find_in_path(ppm_bins)?;

    let prefix = work_dir.join("page");
    let mut ppm_cmd = std::process::Command::new(pdftoppm);
    ppm_cmd
        .args(["-png", "-f", "1", "-l", "3", "-r", "180"])
        .arg(pdf_path)
        .arg(&prefix);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        ppm_cmd.creation_flags(0x0800_0000);
    }
    let ppm_out = ppm_cmd.output().ok()?;
    if !ppm_out.status.success() {
        return None;
    }

    let mut page_images: Vec<PathBuf> = std::fs::read_dir(work_dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        })
        .collect();
    page_images.sort();

    let mut combined = String::new();
    for img in page_images.into_iter().take(3) {
        let mut tcmd = std::process::Command::new(&tess);
        tcmd.arg(&img)
            .arg("stdout")
            .args(["-l", "ind+eng", "--psm", "6"]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            tcmd.creation_flags(0x0800_0000);
        }
        let Ok(out) = tcmd.output() else {
            continue;
        };
        let txt = if out.status.success() {
            String::from_utf8_lossy(&out.stdout).into_owned()
        } else {
            let mut fallback = std::process::Command::new(&tess);
            fallback.arg(&img).arg("stdout").args(["-l", "eng"]);
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                fallback.creation_flags(0x0800_0000);
            }
            fallback
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                .unwrap_or_default()
        };
        if !txt.trim().is_empty() {
            combined.push_str(&txt);
            combined.push('\n');
        }
    }

    (!combined.trim().is_empty()).then_some((combined, "Tesseract OCR".to_string()))
}

#[cfg(target_os = "windows")]
fn try_windows_winrt_ocr(pdf_path: &PathBuf) -> Option<(String, String)> {
    use std::os::windows::process::CommandExt;
    let ps_script = r#"
param([string]$PdfPath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Data.Pdf.PdfDocument, Windows.Data.Pdf, ContentType=WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType=WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation, ContentType=WindowsRuntime]
$null = [Windows.Storage.StorageFile, Windows.Foundation, ContentType=WindowsRuntime]
$null = [Windows.Storage.Streams.InMemoryRandomAccessStream, Windows.Foundation, ContentType=WindowsRuntime]

function Await-WinRT($Op, [Type]$ResultType) {
  $asTask = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1
  } | Select-Object -First 1).MakeGenericMethod($ResultType)
  $t = $asTask.Invoke($null, @($Op))
  return $t.GetAwaiter().GetResult()
}
function Await-WinRTAction($Op) {
  $m = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and -not $_.IsGenericMethod -and $_.GetParameters().Count -eq 1
  } | Select-Object -First 1
  $t = $m.Invoke($null, @($Op))
  $t.GetAwaiter().GetResult()
}

$file = Await-WinRT ([Windows.Storage.StorageFile]::GetFileFromPathAsync($PdfPath)) ([Windows.Storage.StorageFile])
$pdf = Await-WinRT ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
$ocr = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $ocr) { exit 2 }

$pages = [Math]::Min([int]$pdf.PageCount, 3)
$sb = New-Object System.Text.StringBuilder
for ($i = 0; $i -lt $pages; $i++) {
  $page = $pdf.GetPage([uint32]$i)
  $stream = New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
  Await-WinRTAction ($page.RenderToStreamAsync($stream))
  $decoder = Await-WinRT ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
  $bmp = Await-WinRT ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
  $res = Await-WinRT ($ocr.RecognizeAsync($bmp)) ([Windows.Media.Ocr.OcrResult])
  if ($res -and $res.Text) { [void]$sb.AppendLine($res.Text) }
  $stream.Dispose()
  $page.Dispose()
}
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Write-Output $sb.ToString()
"#;

    let script_path = pdf_path.with_extension("ps1");
    std::fs::write(&script_path, ps_script).ok()?;

    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .arg(pdf_path)
        .creation_flags(0x0800_0000)
        .output()
        .ok()?;
    let _ = std::fs::remove_file(&script_path);
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    (!text.trim().is_empty()).then_some((text, "Windows Native WinRT OCR".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_skips_when_text_layer_sufficient() {
        let rich = "Artikel ini membahas penerapan algoritma deep learning dan analisis sentimen ulasan mahasiswa menggunakan arsitektur transformer pada sistem akademik.";
        let res = recover_pdf_text_with_ocr_fallback(b"%PDF-1.4 mock", rich);
        assert!(!res.recovered_via_ocr);
        assert_eq!(res.text, rich);
    }
}

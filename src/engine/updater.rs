//! Lightweight GitHub release checker and SHA-256 verified self-updater for JSFS.
//! Spawns a dedicated background OS thread on startup; completely panic-safe.

#[cfg(target_os = "windows")]
use crate::config::is_dir_writable;
use crate::engine::{EngineEvent, EventSink};
#[cfg(target_os = "windows")]
use std::io::Read;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};

const RELEASE_DOWNLOAD_BASE: &str =
    "https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/download";

/// Compare two semver strings (major.minor.patch). Returns `true` if `latest_tag`
/// is strictly newer than `current_version`. Leading `v` is stripped automatically.
pub fn is_newer_version(latest_tag: &str, current_version: &str) -> bool {
    let latest = latest_tag.trim_start_matches('v');
    let current = current_version.trim_start_matches('v');
    fn parse(s: &str) -> (u32, u32, u32) {
        let mut parts = s.splitn(3, '.').map(|p| p.parse::<u32>().unwrap_or(0));
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        )
    }
    parse(latest) > parse(current)
}

/// Remove any leftover `jsfs.exe.old` created during a previous Windows self-update.
#[cfg(target_os = "windows")]
pub fn cleanup_old_updater_binary() {
    if let Ok(exe_path) = std::env::current_exe() {
        let old_path = old_backup_path(&exe_path);
        if old_path.exists() {
            let _ = std::fs::remove_file(&old_path);
        }
    }
}

/// No-op on Unix platforms (archive updates do not leave `.old` executable backups).
#[cfg(not(target_os = "windows"))]
pub fn cleanup_old_updater_binary() {}

#[cfg(target_os = "windows")]
fn old_backup_path(exe_path: &Path) -> PathBuf {
    let mut os = exe_path.as_os_str().to_os_string();
    os.push(".old");
    PathBuf::from(os)
}

/// Pure-Rust FIPS 180-4 SHA-256 digest returned as lowercase 64-character hex string.
pub fn compute_sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, &word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(word);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut digest = [0u8; 32];
    for (i, val) in h.iter().enumerate() {
        digest[i * 4..(i + 1) * 4].copy_from_slice(&val.to_be_bytes());
    }
    hex::encode(digest)
}

/// Extract expected SHA-256 hex digest for `filename` from `SHA256SUMS.txt`.
pub fn parse_sha256sums(sums_txt: &str, filename: &str) -> Option<String> {
    let target = filename.trim();
    for line in sums_txt.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(hash) = parts.next() else {
            continue;
        };
        let Some(file_part) = parts.next() else {
            continue;
        };
        let clean_file = file_part.trim_start_matches('*');
        if clean_file.eq_ignore_ascii_case(target)
            && hash.len() == 64
            && hash.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

/// Verify downloaded bytes against `SHA256SUMS.txt` entry for `filename`.
pub fn verify_sha256_checksum(data: &[u8], sums_txt: &str, filename: &str) -> Result<(), String> {
    let expected = parse_sha256sums(sums_txt, filename).ok_or_else(|| {
        format!("Hash SHA-256 untuk {filename} tidak ditemukan di SHA256SUMS.txt")
    })?;
    let actual = compute_sha256_hex(data);
    if actual != expected {
        return Err(format!(
            "Verifikasi SHA-256 gagal untuk {filename} (diharapkan {}, didapat {}).",
            &expected[..12],
            &actual[..12]
        ));
    }
    Ok(())
}

/// Spawn a one-shot background update check against the GitHub releases API.
/// Results are delivered via `sink` as `EngineEvent::UpdateAvailable(tag, body_snippet)`.
pub fn spawn_update_check(sink: EventSink) {
    std::thread::Builder::new()
        .name("jsfs-updater".to_string())
        .spawn(move || {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    rt.block_on(async {
                        if let Some((tag, body)) = check_for_update().await {
                            if is_newer_version(&tag, crate::config::APP_VERSION) {
                                sink.send(EngineEvent::UpdateAvailable(tag, body));
                            }
                        }
                    });
                }
            }));
        })
        .ok();
}

/// Spawn background self-update flow for `tag` with SHA-256 verification.
pub fn spawn_self_update(tag: String, sink: EventSink) {
    std::thread::Builder::new()
        .name("jsfs-self-update".to_string())
        .spawn(move || {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let res = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("Gagal inisialisasi runtime pembaruan: {e}"))
                    .and_then(|rt| rt.block_on(perform_self_update(&tag, &sink)));
                if let Err(err) = res {
                    sink.send(EngineEvent::UpdateFailed(err));
                }
            }));
        })
        .ok();
}

#[cfg(target_os = "windows")]
async fn perform_self_update(tag: &str, sink: &EventSink) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .user_agent("JSFS-Self-Updater")
        .build()
        .map_err(|e| format!("Gagal membuat klien HTTP: {e}"))?;

    sink.send(EngineEvent::UpdateProgress(
        "Mengunduh SHA256SUMS.txt...".to_string(),
    ));
    let sums_url = format!("{RELEASE_DOWNLOAD_BASE}/{tag}/SHA256SUMS.txt");
    let sums_resp = client
        .get(&sums_url)
        .send()
        .await
        .map_err(|e| format!("Gagal mengunduh SHA256SUMS.txt: {e}"))?;
    if !sums_resp.status().is_success() {
        return Err(format!(
            "SHA256SUMS.txt belum tersedia (HTTP {}).",
            sums_resp.status()
        ));
    }
    let sums_txt = sums_resp
        .text()
        .await
        .map_err(|e| format!("Gagal membaca SHA256SUMS.txt: {e}"))?;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Gagal mendeteksi lokasi executable: {e}"))?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| "Folder executable tidak valid".to_string())?;
    let writable = is_dir_writable(exe_dir);
    let is_x86 = cfg!(target_arch = "x86");

    if writable {
        let asset_name = if is_x86 {
            "JSFS-windows-x86-portable.zip"
        } else {
            "jsfs.exe"
        };
        sink.send(EngineEvent::UpdateProgress(format!(
            "Mengunduh {asset_name} ({tag})..."
        )));
        let asset_bytes = download_bytes(
            &client,
            &format!("{RELEASE_DOWNLOAD_BASE}/{tag}/{asset_name}"),
        )
        .await?;

        sink.send(EngineEvent::UpdateProgress(
            "Memverifikasi integritas SHA-256...".to_string(),
        ));
        verify_sha256_checksum(&asset_bytes, &sums_txt, asset_name)?;

        let new_exe_bytes = if is_x86 {
            extract_exe_from_zip(&asset_bytes)?
        } else {
            asset_bytes
        };

        sink.send(EngineEvent::UpdateProgress(
            "Memasang biner baru dan memulai ulang JSFS...".to_string(),
        ));
        let old_path = old_backup_path(&current_exe);
        if old_path.exists() {
            let _ = std::fs::remove_file(&old_path);
        }
        std::fs::rename(&current_exe, &old_path)
            .map_err(|e| format!("Gagal mencadangkan biner lama: {e}"))?;
        if let Err(e) = std::fs::write(&current_exe, &new_exe_bytes) {
            let _ = std::fs::rename(&old_path, &current_exe);
            return Err(format!("Gagal menulis biner baru: {e}"));
        }

        std::process::Command::new(&current_exe)
            .spawn()
            .map_err(|e| format!("Biner berhasil diganti, tetapi gagal restart otomatis: {e}"))?;
        std::process::exit(0);
    } else {
        let installer_name = if is_x86 {
            "JSFS-windows-x86-portable.zip"
        } else {
            "JSFS-windows-x64-setup.exe"
        };
        sink.send(EngineEvent::UpdateProgress(format!(
            "Direktori read-only; mengunduh {installer_name}..."
        )));
        let asset_bytes = download_bytes(
            &client,
            &format!("{RELEASE_DOWNLOAD_BASE}/{tag}/{installer_name}"),
        )
        .await?;

        sink.send(EngineEvent::UpdateProgress(
            "Memverifikasi integritas SHA-256...".to_string(),
        ));
        verify_sha256_checksum(&asset_bytes, &sums_txt, installer_name)?;

        let out_path = std::env::temp_dir().join(installer_name);
        std::fs::write(&out_path, &asset_bytes)
            .map_err(|e| format!("Gagal menyimpan installer ke folder sementara: {e}"))?;

        if !is_x86 {
            std::process::Command::new(&out_path)
                .spawn()
                .map_err(|e| format!("Gagal menjalankan installer: {e}"))?;
            std::process::exit(0);
        } else {
            sink.send(EngineEvent::Toast(format!(
                "Pembaruan terverifikasi tersimpan di {}",
                out_path.display()
            )));
            sink.send(EngineEvent::UpdateProgress(String::new()));
            Ok(())
        }
    }
}

#[cfg(unix)]
async fn perform_self_update(tag: &str, sink: &EventSink) -> Result<(), String> {
    let asset_name = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("JSFS-linux-x64.tar.gz")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("JSFS-macos-arm64-preview.tar.gz")
    } else {
        None
    };

    if let Some(name) = asset_name {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .user_agent("JSFS-Self-Updater")
            .build()
            .map_err(|e| format!("Gagal membuat klien HTTP: {e}"))?;

        sink.send(EngineEvent::UpdateProgress(
            "Memeriksa SHA256SUMS.txt untuk platform ini...".to_string(),
        ));
        let sums_url = format!("{RELEASE_DOWNLOAD_BASE}/{tag}/SHA256SUMS.txt");
        if let Ok(resp) = client.get(&sums_url).send().await {
            if resp.status().is_success() {
                if let Ok(sums_txt) = resp.text().await {
                    if parse_sha256sums(&sums_txt, name).is_some() {
                        sink.send(EngineEvent::UpdateProgress(format!(
                            "Mengunduh {name} ({tag})..."
                        )));
                        let bytes = download_bytes(
                            &client,
                            &format!("{RELEASE_DOWNLOAD_BASE}/{tag}/{name}"),
                        )
                        .await?;
                        sink.send(EngineEvent::UpdateProgress(
                            "Memverifikasi integritas SHA-256...".to_string(),
                        ));
                        verify_sha256_checksum(&bytes, &sums_txt, name)?;

                        let out_path = std::env::temp_dir().join(name);
                        std::fs::write(&out_path, &bytes)
                            .map_err(|e| format!("Gagal menyimpan arsip pembaruan: {e}"))?;
                        sink.send(EngineEvent::Toast(format!(
                            "Arsip pembaruan terverifikasi tersimpan di {}",
                            out_path.display()
                        )));
                        sink.send(EngineEvent::UpdateProgress(String::new()));
                        return Ok(());
                    }
                }
            }
        }
    }

    let release_page = format!(
        "https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/{tag}"
    );
    crate::engine::file_ops::open_with_os(&release_page)
        .map_err(|e| format!("Gagal membuka halaman rilis GitHub: {e}"))?;
    sink.send(EngineEvent::Toast(format!(
        "Halaman rilis {tag} dibuka di browser sistem."
    )));
    sink.send(EngineEvent::UpdateProgress(String::new()));
    Ok(())
}

#[cfg(all(not(target_os = "windows"), not(unix)))]
async fn perform_self_update(tag: &str, sink: &EventSink) -> Result<(), String> {
    let release_page = format!(
        "https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/releases/tag/{tag}"
    );
    crate::engine::file_ops::open_with_os(&release_page)
        .map_err(|e| format!("Gagal membuka halaman rilis GitHub: {e}"))?;
    sink.send(EngineEvent::UpdateProgress(String::new()));
    Ok(())
}

async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Gagal mengunduh aset rilis: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Server mengembalikan status HTTP {} saat mengunduh aset.",
            resp.status()
        ));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Gagal membaca stream unduhan: {e}"))?;
    Ok(bytes.to_vec())
}

#[cfg(target_os = "windows")]
fn extract_exe_from_zip(zip_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Gagal membuka arsip ZIP: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Gagal membaca entri ZIP: {e}"))?;
        if file.name().to_ascii_lowercase().ends_with("jsfs.exe") {
            let mut buf = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buf)
                .map_err(|e| format!("Gagal mengekstrak jsfs.exe dari ZIP: {e}"))?;
            return Ok(buf);
        }
    }
    Err("Berkas jsfs.exe tidak ditemukan di dalam arsip ZIP.".to_string())
}

async fn check_for_update() -> Option<(String, String)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("JSFS-Update-Checker")
        .build()
        .ok()?;

    let resp = client
        .get(
            "https://api.github.com/repos/BakwanJagungg/\
             JSFS-Journal-Scraper-For-Students-/releases/latest",
        )
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let tag = json["tag_name"].as_str()?.to_string();
    let body: String = json["body"]
        .as_str()
        .unwrap_or("")
        .chars()
        .take(220)
        .collect();

    Some((tag, body))
}

// ─── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{compute_sha256_hex, is_newer_version, parse_sha256sums, verify_sha256_checksum};

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("v4.11.1", "4.11.0"));
        assert!(is_newer_version("v4.11.0", "4.10.0"));
        assert!(is_newer_version("v4.10.1", "4.10.0"));
        assert!(is_newer_version("v5.0.0", "4.10.0"));
        assert!(!is_newer_version("v4.10.0", "4.10.0"));
        assert!(!is_newer_version("v4.9.1", "4.10.0"));
        assert!(!is_newer_version("v4.10.0", "v4.10.0"));
        assert!(is_newer_version("v4.10.1", "v4.10.0"));
    }

    #[test]
    fn test_sha256_compute_parse_and_verify() {
        let sample = b"JSFS v4.16.0 binary verification payload";
        let digest = compute_sha256_hex(sample);
        assert_eq!(digest.len(), 64);
        // Empty NIST vector sanity check
        assert_eq!(
            compute_sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            compute_sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let sums_txt = format!(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  JSFS-windows-x64-setup.exe\n\
             {digest}  jsfs.exe\n\
             {digest}  JSFS-linux-x64.tar.gz\n\
             {digest}  JSFS-macos-arm64-preview.tar.gz\n"
        );
        assert_eq!(
            parse_sha256sums(&sums_txt, "jsfs.exe"),
            Some(digest.clone())
        );
        assert_eq!(
            parse_sha256sums(&sums_txt, "JSFS-linux-x64.tar.gz"),
            Some(digest.clone())
        );
        assert_eq!(
            parse_sha256sums(&sums_txt, "JSFS-macos-arm64-preview.tar.gz"),
            Some(digest)
        );
        assert!(verify_sha256_checksum(sample, &sums_txt, "jsfs.exe").is_ok());
        assert!(verify_sha256_checksum(sample, &sums_txt, "JSFS-linux-x64.tar.gz").is_ok());
        assert!(
            verify_sha256_checksum(sample, &sums_txt, "JSFS-macos-arm64-preview.tar.gz").is_ok()
        );
        assert!(verify_sha256_checksum(b"tampered", &sums_txt, "jsfs.exe").is_err());
        assert!(verify_sha256_checksum(sample, &sums_txt, "missing.exe").is_err());
    }

    #[test]
    fn test_adaptive_column_width_narrow_vs_wide() {
        let table_w = 780.0_f32;
        let (col_sinta, col_match, col_apc, col_activity, col_action) = if table_w < 880.0 {
            (72.0_f32, 52.0, 126.0, 148.0, 132.0)
        } else {
            (80.0_f32, 62.0, 144.0, 170.0, 146.0)
        };
        assert_eq!(col_sinta, 72.0, "compact: sinta width");
        let fixed_w = col_sinta + col_match + col_apc + col_activity + col_action;
        let col0_w = (table_w - fixed_w).max(220.0);
        assert!(col0_w >= 220.0, "col0 must be at least 220 px on narrow");

        let table_w2 = 1280.0_f32;
        let (col_sinta2, col_match2, col_apc2, col_activity2, col_action2) = if table_w2 < 880.0 {
            (72.0_f32, 52.0, 126.0, 148.0, 132.0)
        } else {
            (80.0_f32, 62.0, 144.0, 170.0, 146.0)
        };
        assert_eq!(col_sinta2, 80.0, "standard: sinta width");
        let fixed_w2 = col_sinta2 + col_match2 + col_apc2 + col_activity2 + col_action2;
        let col0_w2 = (table_w2 - fixed_w2).max(220.0);
        assert!(col0_w2 > 220.0, "col0 must be wide on full-HD");
    }
}

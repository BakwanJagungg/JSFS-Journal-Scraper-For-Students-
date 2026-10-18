"""
ai_installer.py — JSFS v1.4.0 (1-Click AI Setup Manager)
Modul mandiri untuk mengunduh & memasang Ollama serta model AI lokal (qwen2.5:3b)
secara asinkron tanpa memblokir antarmuka GUI.
"""
from __future__ import annotations

import json
import logging
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from typing import Callable, Optional, List, Dict, Any

import requests

log = logging.getLogger("jsfs.ai_installer")

OLLAMA_DOWNLOAD_URL = "https://ollama.com/download/OllamaSetup.exe"
OLLAMA_API_BASE     = "http://localhost:11434"
DEFAULT_MODEL       = "qwen2.5:3b"

# Lokasi standar Ollama di Windows
COMMON_OLLAMA_PATHS = [
    os.path.expandvars(r"%LOCALAPPDATA%\Programs\Ollama\ollama.exe"),
    os.path.expandvars(r"%LOCALAPPDATA%\Programs\Ollama\ollama app.exe"),
    os.path.expandvars(r"%PROGRAMFILES%\Ollama\ollama.exe"),
    os.path.expandvars(r"%PROGRAMFILES(X86)%\Ollama\ollama.exe"),
]


def is_ollama_running(timeout: float = 1.5) -> bool:
    """Mengecek apakah server Ollama sedang aktif dan merespons pada localhost:11434."""
    try:
        r = requests.get(f"{OLLAMA_API_BASE}/api/tags", timeout=timeout)
        return r.status_code == 200
    except (requests.RequestException, Exception):
        return False


def get_installed_models(timeout: float = 2.0) -> List[str]:
    """Mengambil daftar nama model yang telah terpasang di Ollama lokal."""
    try:
        r = requests.get(f"{OLLAMA_API_BASE}/api/tags", timeout=timeout)
        if r.status_code == 200:
            return [m.get("name", "") for m in r.json().get("models", [])]
        return []
    except Exception:
        return []


def is_model_installed(model_name: str = DEFAULT_MODEL, timeout: float = 2.0) -> bool:
    """Mengecek apakah model tertentu sudah diunduh dan tersedia di Ollama."""
    models = get_installed_models(timeout=timeout)
    clean_target = model_name.lower().split(":")[0]
    for m in models:
        m_lower = m.lower()
        if m_lower == model_name.lower():
            return True
        if m_lower == f"{model_name.lower()}:latest":
            return True
        if m_lower.startswith(clean_target + ":"):
            return True
    return False


def find_ollama_executable() -> Optional[str]:
    """Mencari berkas eksekusi Ollama di PATH sistem atau lokasi instalasi standar Windows."""
    # 1. Cek di PATH
    in_path = shutil.which("ollama")
    if in_path and os.path.exists(in_path):
        return in_path

    # 2. Cek lokasi instalasi umum
    for p in COMMON_OLLAMA_PATHS:
        if os.path.exists(p):
            return p
    return None


def launch_ollama_service() -> bool:
    """Jika Ollama terpasang namun belum berjalan, jalankan layanannya."""
    exe_path = find_ollama_executable()
    if not exe_path:
        return False
    try:
        # Jalankan tanpa jendela konsol di Windows
        startupinfo = None
        creationflags = 0
        if sys.platform == "win32":
            startupinfo = subprocess.STARTUPINFO()
            startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            creationflags = subprocess.CREATE_NO_WINDOW | getattr(subprocess, "DETACHED_PROCESS", 0x00000008)

        subprocess.Popen(
            [exe_path, "serve"],
            startupinfo=startupinfo,
            creationflags=creationflags,
            close_fds=True,
        )
        return True
    except Exception as e:
        log.error("Gagal meluncurkan layanan Ollama: %s", e)
        return False


def download_and_install_ollama(
    progress_cb: Callable[[int, int, float, str], None],
    done_cb: Callable[[bool, str], None],
    cancel_evt: Optional[threading.Event] = None,
) -> None:
    """
    Tugas A: Mengunduh OllamaSetup.exe dari situs resmi secara chunked
    dan mengeksekusi installer secara otomatis.
    
    progress_cb signature: (downloaded_bytes, total_bytes, percent, status_text)
    done_cb signature: (success: bool, message: str)
    """
    dest_path = os.path.join(tempfile.gettempdir(), "OllamaSetup.exe")
    try:
        progress_cb(0, 0, 0.0, "Menghubungkan ke server unduhan Ollama...")
        with requests.get(OLLAMA_DOWNLOAD_URL, stream=True, timeout=20) as resp:
            resp.raise_for_status()
            total_bytes = int(resp.headers.get("content-length", 0))
            downloaded = 0
            chunk_size = 256 * 1024  # 256 KB per chunk
            last_update = time.time()

            with open(dest_path, "wb") as f:
                for chunk in resp.iter_content(chunk_size=chunk_size):
                    if cancel_evt and cancel_evt.is_set():
                        done_cb(False, "Pengunduhan Ollama dibatalkan pengguna.")
                        return

                    if chunk:
                        f.write(chunk)
                        downloaded += len(chunk)
                        now = time.time()
                        # Rate limit callback update (setiap 100ms) agar GUI tetap mulus
                        if now - last_update >= 0.1 or (total_bytes > 0 and downloaded >= total_bytes):
                            last_update = now
                            pct = (downloaded / total_bytes * 100) if total_bytes > 0 else 0.0
                            dl_mb = downloaded / (1024 * 1024)
                            tot_mb = total_bytes / (1024 * 1024) if total_bytes > 0 else 0.0
                            status = f"Mengunduh installer: {dl_mb:.1f} MB / {tot_mb:.1f} MB ({pct:.1f}%)"
                            progress_cb(downloaded, total_bytes, pct, status)

        if cancel_evt and cancel_evt.is_set():
            done_cb(False, "Pengunduhan Ollama dibatalkan pengguna.")
            return

        progress_cb(total_bytes, total_bytes, 100.0, "Unduhan selesai. Meluncurkan installer Ollama...")

        # Jalankan installer OllamaSetup.exe
        subprocess.Popen([dest_path], shell=False)
        done_cb(True, "Installer Ollama berhasil diluncurkan! Silakan ikuti wizard instalasi di layar Anda hingga selesai.")

    except Exception as e:
        log.error("Kesalahan saat mengunduh Ollama: %s", e)
        done_cb(False, f"Gagal mengunduh/memasang Ollama: {e}")


def pull_model(
    model_name: str = DEFAULT_MODEL,
    progress_cb: Callable[[int, int, float, str], None] = None,
    done_cb: Callable[[bool, str], None] = None,
    cancel_evt: Optional[threading.Event] = None,
) -> None:
    """
    Tugas B: Melakukan pull model AI ke server Ollama lokal via POST /api/pull.
    Membaca stream JSON line-by-line untuk menghitung persentase progres layer.
    
    progress_cb signature: (completed_bytes, total_bytes, percent, status_text)
    done_cb signature: (success: bool, message: str)
    """
    url = f"{OLLAMA_API_BASE}/api/pull"
    payload = {"name": model_name, "stream": True}

    if not is_ollama_running():
        if done_cb:
            done_cb(False, "Server Ollama belum menyala di localhost:11434. Pastikan aplikasi Ollama berjalan.")
        return

    try:
        if progress_cb:
            progress_cb(0, 0, 0.0, f"Meminta model '{model_name}' dari repositori Ollama...")

        last_update = time.time()
        with requests.post(url, json=payload, stream=True, timeout=120) as resp:
            resp.raise_for_status()

            for line in resp.iter_lines():
                if cancel_evt and cancel_evt.is_set():
                    if done_cb:
                        done_cb(False, f"Pengunduhan model '{model_name}' dibatalkan.")
                    return

                if not line:
                    continue

                try:
                    data = json.loads(line.decode("utf-8", errors="replace"))
                except Exception:
                    continue

                status = data.get("status", "")
                completed = data.get("completed", 0)
                total = data.get("total", 0)

                # Cek jika ada error dari Ollama
                if "error" in data:
                    if done_cb:
                        done_cb(False, f"Ollama Error: {data['error']}")
                    return

                pct = 0.0
                if total > 0:
                    pct = (completed / total) * 100.0

                now = time.time()
                if now - last_update >= 0.15 or status == "success" or pct >= 100.0:
                    last_update = now
                    if total > 0:
                        comp_mb = completed / (1024 * 1024)
                        tot_mb = total / (1024 * 1024)
                        msg = f"{status}: {comp_mb:.1f} MB / {tot_mb:.1f} MB ({pct:.1f}%)"
                    else:
                        msg = f"{status}..."
                    if progress_cb:
                        progress_cb(completed, total, pct, msg)

                if status == "success":
                    if done_cb:
                        done_cb(True, f"Model '{model_name}' berhasil diunduh dan siap digunakan!")
                    return

        # Selesai iterasi tanpa pesan 'success' eksplisit
        if is_model_installed(model_name):
            if done_cb:
                done_cb(True, f"Model '{model_name}' berhasil diverifikasi dan terpasang!")
        else:
            if done_cb:
                done_cb(True, f"Proses pull model selesai.")

    except Exception as e:
        log.error("Kesalahan saat pull model: %s", e)
        if done_cb:
            done_cb(False, f"Gagal mengunduh model '{model_name}': {e}")

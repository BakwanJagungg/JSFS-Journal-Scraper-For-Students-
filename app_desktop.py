"""
app_desktop.py
Process-Isolated App-Mode Desktop Launcher & Localhost Micro-Server untuk JSFS v2.3.0.

Memisahkan proses UI Chromium (Microsoft Edge / Google Chrome `--app=`) sepenuhnya
dari proses backend Python (`ThreadingHTTPServer` + `JSFSApi`), sehingga Windows
tidak pernah menandai jendela UI `(Not Responding)` bahkan saat Python melakukan
komputasi berat (scraping paralel, Match Scoring naskah, SQLite WAL, dan ekspor Excel).
"""

from __future__ import annotations

import base64
import json
import mimetypes
import os
import posixpath
import shutil
import subprocess
import sys
import threading
import time
import urllib.parse
import webbrowser
from email.parser import BytesParser
from email.policy import default as email_default_policy
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Optional

from app_webview import (
    JSFSApi,
    _get_bundle_dir,
    close_forensic_logger,
    init_forensic_logger,
)
from config import APP_VERSION, get_app_dir


_MIME_MAP = {
    ".html": "text/html; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".js": "application/javascript; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".svg": "image/svg+xml",
    ".png": "image/png",
    ".ico": "image/x-icon",
    ".woff2": "font/woff2",
}


class _ThreadedHTTPServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True


def _parse_multipart_file(content_type: str, body: bytes) -> tuple[str, bytes]:
    """Ekstrak (filename, file_bytes) dari payload multipart/form-data tanpa modul cgi."""
    if not content_type or not body:
        return "", b""
    raw_msg = (
        f"Content-Type: {content_type}\r\nMIME-Version: 1.0\r\n\r\n".encode("latin-1", errors="ignore")
        + body
    )
    msg = BytesParser(policy=email_default_policy).parsebytes(raw_msg)
    if not msg.is_multipart():
        return "", b""
    for part in msg.iter_parts():
        fname = part.get_filename()
        if fname:
            payload = part.get_payload(decode=True) or b""
            return str(fname), payload
    return "", b""


def _pick_native_manuscript_dialog() -> dict[str, Any]:
    """Fallback dialog file OS native (tkinter) jika unggah HTML tidak dipakai."""
    try:
        import tkinter as tk
        from tkinter import filedialog
        from doc_reader import extract_manuscript_metadata

        root = tk.Tk()
        root.withdraw()
        try:
            root.attributes("-topmost", True)
        except Exception:
            pass
        try:
            path = filedialog.askopenfilename(
                parent=root,
                title="Pilih Naskah Artikel Ilmiah (.docx / .pdf)",
                filetypes=[
                    ("Dokumen Naskah", "*.docx *.pdf"),
                    ("Semua Berkas", "*.*"),
                ],
            )
        finally:
            root.destroy()

        if not path:
            return {"ok": False, "path": ""}
        suggested = ""
        title = ""
        try:
            meta = extract_manuscript_metadata(path)
            kws = meta.get("keywords") or []
            if kws:
                suggested = ", ".join(kws[:3])
            title = str(meta.get("title") or "")
        except Exception:
            pass
        return {
            "ok": True,
            "path": path,
            "name": os.path.basename(path),
            "suggested_keyword": suggested,
            "title": title,
        }
    except Exception as exc:
        return {"ok": False, "path": "", "message": str(exc)}


def find_app_browser_executable() -> Optional[str]:
    """Cari executable browser berbasis Chromium (Edge / Chrome / Brave) yang mendukung flag `--app=`."""
    if sys.platform == "win32":
        env_dirs = [
            os.environ.get("PROGRAMFILES(X86)", ""),
            os.environ.get("PROGRAMFILES", ""),
            os.environ.get("LOCALAPPDATA", ""),
        ]
        rel_candidates = [
            os.path.join("Microsoft", "Edge", "Application", "msedge.exe"),
            os.path.join("Google", "Chrome", "Application", "chrome.exe"),
            os.path.join("BraveSoftware", "Brave-Browser", "Application", "brave.exe"),
        ]
        for base in env_dirs:
            if not base:
                continue
            for rel in rel_candidates:
                candidate = os.path.join(base, rel)
                if os.path.isfile(candidate):
                    return candidate

    for name in ("msedge.exe", "msedge", "chrome.exe", "google-chrome", "chromium", "brave.exe", "brave-browser"):
        found = shutil.which(name)
        if found:
            return found
    return None


class JSFSDesktopServer:
    """Server HTTP localhost terisolasi yang menjembatani UI App-Mode dengan JSFSApi."""

    def __init__(self, api: Optional[JSFSApi] = None, host: str = "127.0.0.1", port: int = 0):
        self.api = api if api is not None else JSFSApi()
        self.host = host
        self.requested_port = int(port)
        self.bundle_dir = os.path.abspath(_get_bundle_dir())
        self.ui_dir = os.path.join(self.bundle_dir, "ui_web")
        self.assets_dir = os.path.join(self.bundle_dir, "assets")
        self.shutdown_event = threading.Event()
        self._state_lock = threading.Lock()
        self._seen_client = False
        self._last_ping_mono = time.monotonic()
        self._httpd: Optional[_ThreadedHTTPServer] = None
        self._server_thread: Optional[threading.Thread] = None
        self.port: int = 0

    @property
    def url(self) -> str:
        return f"http://{self.host}:{self.port}"

    def mark_ping(self) -> float:
        now = time.monotonic()
        with self._state_lock:
            self._seen_client = True
            self._last_ping_mono = now
        return now

    def heartbeat_timed_out(self, timeout_sec: float = 12.0) -> bool:
        with self._state_lock:
            if not self._seen_client:
                return False
            return (time.monotonic() - self._last_ping_mono) > timeout_sec

    def start(self) -> int:
        if self._httpd is not None:
            return self.port
        handler_cls = self._build_handler_class()
        self._httpd = _ThreadedHTTPServer((self.host, self.requested_port), handler_cls)
        self.port = int(self._httpd.server_address[1])
        self._server_thread = threading.Thread(
            target=self._httpd.serve_forever,
            kwargs={"poll_interval": 0.15},
            daemon=True,
            name="JSFS-LocalHTTPServer",
        )
        self._server_thread.start()
        return self.port

    def stop(self) -> None:
        self.shutdown_event.set()
        httpd = self._httpd
        self._httpd = None
        if httpd is not None:
            try:
                httpd.shutdown()
            except Exception:
                pass
            try:
                httpd.server_close()
            except Exception:
                pass
        if self._server_thread is not None and self._server_thread.is_alive():
            self._server_thread.join(timeout=0.4)
        self.api.shutdown()

    def _build_handler_class(self):
        parent = self

        class JSFSRequestHandler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def log_message(self, format: str, *args) -> None:
                # Hindari penulisan log HTTP ke stderr (penting saat console=False di PyInstaller)
                return

            def _send_json(self, payload: Any, status: int = HTTPStatus.OK) -> None:
                raw = json.dumps(payload, ensure_ascii=False).encode("utf-8")
                self.send_response(status)
                self.send_header("Content-Type", "application/json; charset=utf-8")
                self.send_header("Content-Length", str(len(raw)))
                self.send_header("Cache-Control", "no-store, no-cache, must-revalidate")
                self.end_headers()
                self.wfile.write(raw)

            def _read_body(self) -> bytes:
                try:
                    length = int(self.headers.get("Content-Length", "0") or "0")
                except ValueError:
                    length = 0
                if length <= 0:
                    return b""
                # Batas aman 64 MB untuk unggahan naskah .docx/.pdf
                if length > 64 * 1024 * 1024:
                    raise ValueError("Ukuran payload melebihi batas maksimal 64 MB.")
                return self.rfile.read(length)

            def _read_json_body(self) -> dict[str, Any]:
                raw = self._read_body()
                if not raw:
                    return {}
                try:
                    data = json.loads(raw.decode("utf-8"))
                    return data if isinstance(data, dict) else {}
                except Exception:
                    return {}

            def _serve_static_file(self, abs_path: str) -> None:
                if not os.path.isfile(abs_path):
                    self._send_json({"ok": False, "error": "Berkas tidak ditemukan"}, HTTPStatus.NOT_FOUND)
                    return
                ext = os.path.splitext(abs_path)[1].lower()
                ctype = _MIME_MAP.get(ext) or mimetypes.guess_type(abs_path)[0] or "application/octet-stream"
                with open(abs_path, "rb") as fh:
                    data = fh.read()
                self.send_response(HTTPStatus.OK)
                self.send_header("Content-Type", ctype)
                self.send_header("Content-Length", str(len(data)))
                self.send_header("Cache-Control", "no-store, no-cache, must-revalidate")
                self.end_headers()
                self.wfile.write(data)

            def do_GET(self) -> None:
                parsed = urllib.parse.urlparse(self.path)
                route = parsed.path

                if route == "/api/init":
                    info = parent.api.get_init_info()
                    info["mode"] = "app_desktop"
                    info["port"] = parent.port
                    parent.mark_ping()
                    self._send_json(info)
                    return

                if route == "/api/status":
                    qs = urllib.parse.parse_qs(parsed.query)
                    try:
                        log_offset = int((qs.get("log_offset") or ["0"])[0])
                    except (TypeError, ValueError):
                        log_offset = 0
                    try:
                        last_ver = int((qs.get("last_result_version") or ["-1"])[0])
                    except (TypeError, ValueError):
                        last_ver = -1
                    self._send_json(parent.api.get_status(log_offset, last_ver))
                    return

                if route == "/api/ai-status":
                    self._send_json(parent.api.get_ai_setup_status())
                    return

                if route == "/favicon.ico":
                    self._serve_static_file(os.path.join(parent.assets_dir, "icon.ico"))
                    return

                if route.startswith("/assets/"):
                    rel_asset = posixpath.normpath(urllib.parse.unquote(route[len("/assets/"):])).lstrip("/")
                    if ".." in rel_asset.split("/"):
                        self._send_json({"ok": False, "error": "Forbidden"}, HTTPStatus.FORBIDDEN)
                        return
                    target = os.path.abspath(os.path.join(parent.assets_dir, rel_asset))
                    if not target.startswith(os.path.abspath(parent.assets_dir)):
                        self._send_json({"ok": False, "error": "Forbidden"}, HTTPStatus.FORBIDDEN)
                        return
                    self._serve_static_file(target)
                    return

                if route in ("/", ""):
                    rel_ui = "index.html"
                else:
                    rel_ui = posixpath.normpath(urllib.parse.unquote(route)).lstrip("/")
                if ".." in rel_ui.split("/"):
                    self._send_json({"ok": False, "error": "Forbidden"}, HTTPStatus.FORBIDDEN)
                    return
                target = os.path.abspath(os.path.join(parent.ui_dir, rel_ui))
                if not target.startswith(os.path.abspath(parent.ui_dir)):
                    self._send_json({"ok": False, "error": "Forbidden"}, HTTPStatus.FORBIDDEN)
                    return
                self._serve_static_file(target)

            def do_POST(self) -> None:
                route = urllib.parse.urlparse(self.path).path

                if route == "/api/ping":
                    self._read_body()
                    ts = parent.mark_ping()
                    self._send_json({"ok": True, "ts": round(ts, 3)})
                    return

                if route == "/api/shutdown":
                    self._read_body()
                    parent.shutdown_event.set()
                    self._send_json({"ok": True})
                    return

                if route == "/api/upload-manuscript":
                    try:
                        raw_body = self._read_body()
                    except ValueError as exc:
                        self._send_json({"ok": False, "message": str(exc)}, HTTPStatus.REQUEST_ENTITY_TOO_LARGE)
                        return

                    ctype = (self.headers.get("Content-Type") or "").strip()
                    x_filename = urllib.parse.unquote(self.headers.get("X-Filename") or "").strip()

                    if ctype.lower().startswith("multipart/form-data"):
                        fname, fbytes = _parse_multipart_file(ctype, raw_body)
                        self._send_json(parent.api.upload_manuscript_bytes(fname, fbytes))
                        return

                    if x_filename and raw_body:
                        self._send_json(parent.api.upload_manuscript_bytes(x_filename, raw_body))
                        return

                    if raw_body and (ctype.lower().startswith("application/json") or raw_body.lstrip().startswith(b"{")):
                        try:
                            data = json.loads(raw_body.decode("utf-8"))
                        except Exception:
                            data = {}
                        if isinstance(data, dict) and data.get("data_base64") and data.get("filename"):
                            try:
                                raw_file = base64.b64decode(str(data["data_base64"]))
                            except Exception as exc:
                                self._send_json({"ok": False, "message": f"Base64 tidak valid: {exc}"}, HTTPStatus.BAD_REQUEST)
                                return
                            self._send_json(parent.api.upload_manuscript_bytes(str(data["filename"]), raw_file))
                            return

                    self._send_json(_pick_native_manuscript_dialog())
                    return

                payload = self._read_json_body()
                dispatch = {
                    "/api/search": lambda: parent.api.start_search(payload),
                    "/api/stop": lambda: parent.api.stop_scrape(),
                    "/api/copy-share": lambda: parent.api.copy_share_list(payload.get("indices")),
                    "/api/copy-draft": lambda: parent.api.copy_editor_draft(
                        int(payload.get("index", -1)),
                        str(payload.get("keyword", "") or ""),
                    ),
                    "/api/open-excel": lambda: parent.api.open_excel(),
                    "/api/open-output": lambda: parent.api.open_output_folder(),
                    "/api/open-logs": lambda: parent.api.open_logs_folder(),
                    "/api/open-url": lambda: parent.api.open_url(str(payload.get("url", "") or "")),
                    "/api/clear-cache": lambda: parent.api.clear_cache(),
                    "/api/launch-ollama": lambda: parent.api.launch_ollama(),
                    "/api/install-ollama": lambda: parent.api.install_ollama(),
                    "/api/pull-ai-model": lambda: parent.api.pull_ai_model(),
                }
                handler_fn = dispatch.get(route)
                if handler_fn is None:
                    self._send_json({"ok": False, "error": f"Endpoint tidak dikenal: {route}"}, HTTPStatus.NOT_FOUND)
                    return
                try:
                    result = handler_fn()
                    self._send_json(result)
                except Exception as exc:
                    self._send_json({"ok": False, "message": str(exc)}, HTTPStatus.INTERNAL_SERVER_ERROR)

        return JSFSRequestHandler


def launch_app_window(url: str) -> Optional[subprocess.Popen]:
    """Meluncurkan jendela Chromium terisolasi (`--app=`, `1160x780`, tanpa address bar/tab)."""
    browser_exe = find_app_browser_executable()
    if browser_exe:
        profile_dir = os.path.join(get_app_dir(), ".cache", "app_profile")
        os.makedirs(profile_dir, exist_ok=True)
        cmd = [
            browser_exe,
            f"--app={url}",
            "--window-size=1160,780",
            f"--user-data-dir={profile_dir}",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-extensions",
            "--new-window",
        ]
        try:
            return subprocess.Popen(cmd)
        except Exception:
            pass
    webbrowser.open(url)
    return None


def main() -> None:
    init_forensic_logger()
    server = JSFSDesktopServer()
    try:
        server.start()
        server.api.startup_background_checks()
        launch_mono = time.monotonic()
        proc = launch_app_window(server.url)

        while not server.shutdown_event.is_set():
            if proc is not None:
                ret = proc.poll()
                if ret is not None:
                    # Jika proses browser hidup >= 2.5 detik lalu keluar, user menutup jendela app
                    if (time.monotonic() - launch_mono) >= 2.5:
                        break
                    # Jika browser langsung me-delegate ke instance existing < 2.5s, andalkan beacon/heartbeat
                    proc = None

            if server.heartbeat_timed_out(timeout_sec=12.0):
                break

            server.shutdown_event.wait(timeout=0.15)
    except KeyboardInterrupt:
        pass
    finally:
        server.stop()
        close_forensic_logger()


if __name__ == "__main__":
    main()

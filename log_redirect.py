# =============================================================================
# JSFS - Journal Scraper For Students
# log_redirect.py — Interceptor output terminal & rich.Console ke queue.Queue
# =============================================================================

import io
import queue
import re
import sys
from typing import Optional
from rich.console import Console


class QueueWriter(io.TextIOBase):
    """
    Custom IO writer yang menangkap seluruh output teks dan meneruskannya ke queue.Queue.
    Membersihkan ANSI escape sequences sehingga log yang masuk ke antarmuka GUI
    rapi dan bebas dari karakter aneh.
    """

    def __init__(self, log_queue: queue.Queue):
        super().__init__()
        self.queue = log_queue

    def write(self, text: str) -> int:
        if not text:
            return 0
        # Hapus ANSI escape sequences (warna & format terminal)
        clean = re.sub(r'\x1b\[[0-9;]*[a-zA-Z]', '', text)
        # Pisahkan per baris agar log masuk secara atomik
        for line in clean.splitlines():
            stripped = line.strip()
            if stripped:
                self.queue.put(stripped)
        return len(text)

    def flush(self):
        """No-op untuk memenuhi kontrak TextIOBase."""
        pass


def make_queue_console(log_queue: queue.Queue) -> Console:
    """
    Buat instance rich.Console yang output-nya diarahkan ke QueueWriter.
    Dengan color_system=None, Rich tidak akan menginjeksi escape sequence warna.
    """
    writer = QueueWriter(log_queue)
    return Console(
        file=writer,
        highlight=False,
        markup=True,
        force_terminal=False,
        color_system=None,
    )

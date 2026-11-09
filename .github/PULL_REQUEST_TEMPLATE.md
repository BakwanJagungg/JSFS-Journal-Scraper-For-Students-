## 📋 Deskripsi Pull Request
<!-- Jelaskan secara singkat fitur baru atau perbaikan bug yang dilakukan -->

### 🎯 Terkait dengan Issue:
<!-- Contoh: Fixes #12 atau Closes #5 -->

---

### 🛡️ Pre-Flight Verification Checklist (Wajib Sesuai VISION.md):
- [ ] **Zero Heavy ML**: Tidak menambahkan library berat (`torch`, `tensorflow`, `scipy`, `scikit-learn`).
- [ ] **Ukuran Biner**: Memastikan ukuran biner `dist/JSFS.exe` tetap di bawah **~280 MB**.
- [ ] **Thread-Safe GUI**: Tidak melakukan mutasi widget CustomTkinter dari *background thread* (selalu melalui `queue.Queue`).
- [ ] **Anti-OA-Trap**: Mempertahankan filter Open Access pembaca agar tidak melabeli jurnal sebagai Gratis.
- [ ] **Syntax & Unit Test**: Seluruh unit test di `tests/test_jsfs_core.py` lulus 100% dan `python -m py_compile *.py` menghasilkan 0 error.


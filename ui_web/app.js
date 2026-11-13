(function () {
  "use strict";

  const MONTH_NAMES = ["Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des"];
  const collator = new Intl.Collator("id", { numeric: true, sensitivity: "base" });

  const state = {
    manuscriptPath: "",
    running: false,
    results: [],
    resultsVersion: -1,
    selectedIndex: -1,
    excelPath: "",
    activePreset: "skripsi",
    pollTimer: null,
    aiPollTimer: null,
    logOffset: 0,
    detailIndex: -1,
  };

  function $(id) {
    return document.getElementById(id);
  }

  function hasApi() {
    return !!(window.pywebview && window.pywebview.api);
  }

  function showToast(msg) {
    const el = $("toast");
    if (!el) return;
    el.textContent = msg;
    el.classList.remove("is-hidden");
    clearTimeout(showToast._timer);
    showToast._timer = setTimeout(() => {
      el.classList.add("is-hidden");
    }, 2600);
  }

  function appendLog(text) {
    const box = $("activity-log");
    if (!box) return;
    const line = document.createElement("div");
    line.className = "log-line";
    if (text.includes("⚠️") || text.includes("⚠")) line.classList.add("is-warn");
    else if (text.includes("❌") || text.includes("Error")) line.classList.add("is-err");
    else if (text.includes("✅") || text.includes("💾") || text.includes("Selesai")) line.classList.add("is-ok");
    line.textContent = text;
    box.appendChild(line);
    box.scrollTop = box.scrollHeight;
  }

  // ── Theme Toggle (Dark / Daylight) ───────────────────────────────────────────
  function syncThemeUI() {
    const meta = document.querySelector('meta[name="color-scheme"]');
    const cur = meta ? meta.getAttribute("content") : "light";
    const isDark = cur === "dark";
    const icon = $("theme-icon");
    const label = $("theme-label");
    if (icon) icon.textContent = isDark ? "☀️" : "🌙";
    if (label) label.textContent = isDark ? "Daylight" : "Mode Gelap";
  }

  function toggleTheme() {
    const meta = document.querySelector('meta[name="color-scheme"]');
    if (!meta) return;
    const isDark = meta.getAttribute("content") === "dark";
    const next = isDark ? "light" : "dark";
    meta.setAttribute("content", next);
    try {
      localStorage.setItem("color-scheme", next);
    } catch (e) {}
    syncThemeUI();
  }

  // ── Native <dialog> Handling ─────────────────────────────────────────────────
  function openDialog(id) {
    const dlg = $(id);
    if (dlg && typeof dlg.showModal === "function" && !dlg.open) {
      dlg.showModal();
    }
  }

  function setupDialogs() {
    document.querySelectorAll("dialog.modal-dialog").forEach((dlg) => {
      dlg.addEventListener("close", () => {
        dlg.returnValue = "";
      });
      dlg.addEventListener("click", (ev) => {
        if (ev.target === dlg) {
          dlg.close();
        }
      });
    });

    document.querySelectorAll("[data-close-dialog]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const dlg = $(btn.getAttribute("data-close-dialog"));
        if (dlg && dlg.open) dlg.close();
      });
    });
  }

  // ── Presets & Form Helpers ───────────────────────────────────────────────────
  function setSintaChecks(ranks) {
    document.querySelectorAll('input[name="sinta"]').forEach((cb) => {
      cb.checked = ranks.includes(Number(cb.value));
    });
  }

  function setMonthChecks(months) {
    document.querySelectorAll('input[name="month"]').forEach((cb) => {
      cb.checked = months.includes(Number(cb.value));
    });
  }

  function getNearThreeMonths() {
    const cur = new Date().getMonth() + 1;
    return [cur, (cur % 12) + 1, ((cur + 1) % 12) + 1];
  }

  function applyPreset(key, silent) {
    state.activePreset = key;
    document.querySelectorAll(".preset-card").forEach((card) => {
      card.classList.toggle("is-active", card.getAttribute("data-preset") === key);
    });

    const pill = $("active-preset-pill");
    if (key === "skripsi") {
      setSintaChecks([3, 4, 5]);
      $("select-target").value = "25";
      $("chk-only-free").checked = false;
      $("chk-include-na").checked = true;
      $("input-min-rp").value = "0";
      $("input-max-rp").value = "1.500.000";
      setMonthChecks([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
      if (pill) pill.textContent = "Preset Aktif: 🎓 Target Skripsi S1 (S3–S5)";
      if (!silent) appendLog("📌 Preset Aktif: 🎓 Target Skripsi S1 (SINTA 3–5, Target 25, Biaya maks Rp 1,5 jt).");
    } else if (key === "gratis") {
      setSintaChecks([2, 3, 4, 5, 6]);
      $("select-target").value = "50";
      $("chk-only-free").checked = true;
      $("chk-include-na").checked = true;
      setMonthChecks([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
      if (pill) pill.textContent = "Preset Aktif: 💰 Jurnal Gratis (Rp 0)";
      if (!silent) appendLog("📌 Preset Aktif: 💰 Jurnal Bebas Biaya (Bebas APC, SINTA 2–6, Target 50).");
    } else if (key === "cepat") {
      setSintaChecks([3, 4, 5, 6]);
      $("select-target").value = "25";
      $("chk-only-free").checked = false;
      $("chk-include-na").checked = true;
      const near = getNearThreeMonths();
      setMonthChecks(near);
      const names = near.slice().sort((a, b) => a - b).map((m) => MONTH_NAMES[m - 1]).join(", ");
      if (pill) pill.textContent = `Preset Aktif: ⚡ Terbit Terdekat (${names})`;
      if (!silent) appendLog(`📌 Preset Aktif: ⚡ Terbit Terdekat (${names}, SINTA 3–6, Target 25).`);
    } else if (key === "reputasi") {
      setSintaChecks([1, 2]);
      $("select-target").value = "25";
      $("chk-only-free").checked = false;
      $("chk-include-na").checked = true;
      setMonthChecks([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
      if (pill) pill.textContent = "Preset Aktif: 🏆 Reputasi Tinggi (S1–S2)";
      if (!silent) appendLog("📌 Preset Aktif: 🏆 Reputasi Tinggi (SINTA 1 & 2, Target 25).");
    }
  }

  function parseCurrency(raw) {
    const digits = String(raw || "").replace(/\D+/g, "");
    return digits ? Number(digits) : 0;
  }

  function collectParams() {
    const sinta = Array.from(document.querySelectorAll('input[name="sinta"]:checked')).map((c) => Number(c.value));
    const months = Array.from(document.querySelectorAll('input[name="month"]:checked')).map((c) => Number(c.value));
    const formats = Array.from(document.querySelectorAll('input[name="fmt"]:checked')).map((c) => c.value);

    return {
      keyword: $("input-keyword").value.trim(),
      manuscript: state.manuscriptPath,
      sinta: sinta.length ? sinta : [1, 2, 3, 4, 5, 6],
      months: months.length ? months : [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
      include_unknown: $("chk-include-unknown-month").checked,
      min_rp: parseCurrency($("input-min-rp").value),
      max_rp: parseCurrency($("input-max-rp").value) || 15000000,
      target: Number($("select-target").value) || 25,
      garuda: $("chk-garuda").checked,
      sinta_src: $("chk-sinta-src").checked,
      only_free: $("chk-only-free").checked,
      include_na: $("chk-include-na").checked,
      match: $("chk-match").checked,
      turbo: $("chk-turbo").checked,
      selenium: $("chk-selenium").checked,
      ai_enabled: $("chk-ai").checked,
      ai_workers: Number($("select-ai-workers").value) || 2,
      formats: formats.length ? formats : [".xlsx", ".md"],
    };
  }

  // ── Render Results Table & Sortable Columns ──────────────────────────────────
  function classifyApc(item) {
    const cat = item.apc_category;
    if (cat === "free" || cat === "paid" || cat === "unknown") return cat;
    const amt = item.biaya_publikasi;
    const txt = String(item.apc_label || "").toLowerCase();
    if (amt === 0 || txt.includes("gratis") || txt.includes("rp 0")) return "free";
    if (typeof amt === "number" && amt > 0) return "paid";
    return "unknown";
  }

  function renderResultsTable(items) {
    const tbody = $("results-tbody");
    if (!tbody) return;
    tbody.innerHTML = "";
    state.selectedIndex = -1;
    $("btn-copy-editor").disabled = true;

    if (!items || items.length === 0) {
      const tr = document.createElement("tr");
      tr.className = "empty-row";
      const td = document.createElement("td");
      td.colSpan = 7;
      td.innerHTML = `
        <div class="empty-state">
          <div class="empty-icon" aria-hidden="true">📚</div>
          <p><strong>Belum ada jurnal yang ditampilkan.</strong></p>
          <p class="muted">Coba kata kunci lain atau periksa kembali filter SINTA/Bulan Anda.</p>
        </div>`;
      tr.appendChild(td);
      tbody.appendChild(tr);
      $("btn-copy-wa").disabled = true;
      updateVisibleRowNumbers();
      return;
    }

    $("btn-copy-wa").disabled = false;

    items.forEach((item, idx) => {
      const tr = document.createElement("tr");
      const apcCat = classifyApc(item);
      tr.dataset.idx = String(idx);
      tr.dataset.apc = apcCat;
      tr.dataset.search = [
        item.nama,
        item.penerbit,
        item.focus_scope,
        item.email,
        item.telepon,
        item.apc_label,
        item.jadwal_terbit,
      ].join(" ").toLowerCase();
      tr.dataset.sortNama = String(item.nama || "");
      tr.dataset.sortSinta = String(item.sinta || 99);
      tr.dataset.sortMatch_score = String(item.match_score ?? 0);
      tr.dataset.sortBiaya_sort = String(item.biaya_sort ?? 999999999);
      tr.dataset.sortJadwal_sort = String(item.terbit_terdekat || "zzz");

      // #
      const tdNum = document.createElement("td");
      tdNum.className = "col-num";
      tdNum.textContent = String(idx + 1);

      // Nama & Penerbit
      const tdName = document.createElement("td");
      const nm = document.createElement("span");
      nm.className = "j-name";
      nm.textContent = item.nama || "Tanpa Nama";
      tdName.appendChild(nm);

      const pub = document.createElement("div");
      pub.className = "j-pub";
      pub.textContent = `${item.penerbit || "-"} • ${item.sumber || "Portal"}`;
      tdName.appendChild(pub);

      if (item.health_warning) {
        const warn = document.createElement("span");
        warn.className = "j-warn";
        warn.textContent = "⚠ Data kontak/jadwal minim — cek situs";
        tdName.appendChild(warn);
      }

      // SINTA
      const tdSinta = document.createElement("td");
      const badgeSinta = document.createElement("span");
      badgeSinta.className = "badge badge-sinta";
      badgeSinta.textContent = item.sinta ? `SINTA ${item.sinta}` : "Garuda";
      tdSinta.appendChild(badgeSinta);

      // Match
      const tdMatch = document.createElement("td");
      const scoreEl = document.createElement("div");
      scoreEl.className = "match-score";
      scoreEl.textContent = `${item.match_score ?? 40}%`;
      const levelEl = document.createElement("div");
      levelEl.className = "muted";
      levelEl.textContent = item.kesesuaian_naskah || "Cukup Sesuai";
      tdMatch.append(scoreEl, levelEl);

      // Biaya APC
      const tdApc = document.createElement("td");
      const badgeApc = document.createElement("span");
      badgeApc.className =
        apcCat === "free" ? "badge badge-free" : apcCat === "paid" ? "badge badge-paid" : "badge badge-unk";
      badgeApc.textContent = item.apc_label || "Cek Web / Belum Terdata";
      tdApc.appendChild(badgeApc);

      if (item.submit_label && item.submit_label !== "-") {
        const subNote = document.createElement("div");
        subNote.className = "muted";
        subNote.textContent = `Submit: ${item.submit_label}`;
        tdApc.appendChild(subNote);
      }

      // Jadwal Terbit
      const tdJadwal = document.createElement("td");
      const jadTop = document.createElement("div");
      jadTop.textContent = item.jadwal_terbit || "-";
      const jadEst = document.createElement("div");
      jadEst.className = "muted";
      jadEst.textContent = `Terdekat: ${item.terbit_terdekat || "-"}`;
      tdJadwal.append(jadTop, jadEst);

      // Kontak & Aksi
      const tdAct = document.createElement("td");
      const contactLine = document.createElement("div");
      contactLine.className = "muted";
      contactLine.textContent = item.email_short || "Email belum terdata";
      tdAct.appendChild(contactLine);

      const actWrap = document.createElement("div");
      actWrap.className = "cell-actions";

      const btnDetail = document.createElement("button");
      btnDetail.type = "button";
      btnDetail.className = "btn btn-secondary btn-xs";
      btnDetail.textContent = "🔎 Detail";
      btnDetail.addEventListener("click", (ev) => {
        ev.stopPropagation();
        selectRow(idx);
        openJournalDetail(idx);
      });
      actWrap.appendChild(btnDetail);

      if (item.url) {
        const btnWeb = document.createElement("button");
        btnWeb.type = "button";
        btnWeb.className = "btn btn-ghost btn-xs";
        btnWeb.textContent = "🌐 Web";
        btnWeb.addEventListener("click", (ev) => {
          ev.stopPropagation();
          if (hasApi()) window.pywebview.api.open_url(item.url);
        });
        actWrap.appendChild(btnWeb);
      }
      tdAct.appendChild(actWrap);

      tr.append(tdNum, tdName, tdSinta, tdMatch, tdApc, tdJadwal, tdAct);
      tr.addEventListener("click", () => selectRow(idx));
      tbody.appendChild(tr);
    });

    applyTableFilters();
  }

  function selectRow(idx) {
    state.selectedIndex = idx;
    document.querySelectorAll("#results-tbody tr").forEach((tr) => {
      tr.classList.toggle("is-selected", Number(tr.dataset.idx) === idx);
    });
    $("btn-copy-editor").disabled = idx < 0;
  }

  function updateVisibleRowNumbers() {
    let visible = 0;
    const rows = document.querySelectorAll("#results-tbody tr:not(.empty-row)");
    rows.forEach((tr) => {
      if (!tr.classList.contains("is-hidden")) {
        visible += 1;
        const numCell = tr.querySelector("td.col-num");
        if (numCell) numCell.textContent = String(visible);
      }
    });
    const total = state.results.length;
    const pill = $("results-count-pill");
    if (pill) {
      pill.textContent = visible === total ? `${total} jurnal` : `${visible} dari ${total} jurnal`;
    }
  }

  function applyTableFilters() {
    const q = ($("filter-search").value || "").trim().toLowerCase();
    const apc = $("filter-apc").value || "all";

    document.querySelectorAll("#results-tbody tr:not(.empty-row)").forEach((tr) => {
      const matchText = !q || (tr.dataset.search || "").includes(q);
      const matchApc = apc === "all" || tr.dataset.apc === apc;
      tr.classList.toggle("is-hidden", !(matchText && matchApc));
    });
    updateVisibleRowNumbers();
  }

  function sortTableBy(colKey, sortType, thEl) {
    const tbody = $("results-tbody");
    if (!tbody || !state.results.length) return;

    const currentSort = thEl.getAttribute("aria-sort");
    const nextDir = currentSort === "ascending" ? "descending" : "ascending";

    document.querySelectorAll("#results-table thead th[aria-sort]").forEach((th) => {
      th.setAttribute("aria-sort", th === thEl ? nextDir : "none");
    });

    const dataKey = "sort" + colKey.charAt(0).toUpperCase() + colKey.slice(1);
    const rows = Array.from(tbody.querySelectorAll("tr:not(.empty-row)"));
    rows.sort((a, b) => {
      const va = a.dataset[dataKey] ?? "";
      const vb = b.dataset[dataKey] ?? "";
      let cmp = 0;
      if (sortType === "num") {
        cmp = (Number(va) || 0) - (Number(vb) || 0);
      } else {
        cmp = collator.compare(va, vb);
      }
      return nextDir === "ascending" ? cmp : -cmp;
    });

    rows.forEach((row) => {
      if (typeof tbody.moveBefore === "function") {
        tbody.moveBefore(row, null);
      } else {
        tbody.appendChild(row);
      }
    });
    updateVisibleRowNumbers();
  }

  function openJournalDetail(idx) {
    const item = state.results[idx];
    if (!item) return;
    state.detailIndex = idx;
    $("detail-title").textContent = item.nama || "Detail Jurnal";
    const body = $("detail-body");
    body.innerHTML = "";

    const fields = [
      ["Penerbit", item.penerbit || "-"],
      ["Peringkat & Sumber", `${item.sinta ? "SINTA " + item.sinta : "Garuda"} • ${item.sumber || "-"}`],
      ["Biaya Publikasi (APC)", item.apc_label || "-"],
      ["Biaya Submit", item.submit_label || "-"],
      ["Jadwal Terbit", `${item.jadwal_terbit || "-"} (Terdekat: ${item.terbit_terdekat || "-"})`],
      ["Skor Kesesuaian", `${item.match_score ?? 40}% (${item.kesesuaian_naskah || "-"})`],
      ["Email Redaksi", item.email || "-"],
      ["Telepon / WA", item.telepon || "-"],
      ["Focus & Scope", item.focus_scope || "-", true],
      ["Catatan & Tautan Biaya", item.apc_url || item.url || "-", true],
    ];

    const grid = document.createElement("div");
    grid.className = "detail-grid";
    fields.forEach(([k, v, full]) => {
      const box = document.createElement("div");
      box.className = full ? "detail-item full" : "detail-item";
      const dk = document.createElement("div");
      dk.className = "detail-k";
      dk.textContent = k;
      const dv = document.createElement("div");
      dv.className = "detail-v";
      dv.textContent = v;
      box.append(dk, dv);
      grid.appendChild(box);
    });
    body.appendChild(grid);
    openDialog("dialog-detail");
  }

  // ── Backend Sync & Actions ───────────────────────────────────────────────────
  function updateProgressBar(prefix, done, total) {
    const pct = total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 0;
    $(`${prefix}-prog-text`).textContent = `${done} / ${total}`;
    $(`${prefix}-prog-fill`).style.width = `${pct}%`;
    const wrap = $(`${prefix}-prog-bar-wrap`);
    if (wrap) wrap.setAttribute("aria-valuenow", String(pct));
  }

  function updateRunningUI(running) {
    state.running = running;
    $("btn-start").classList.toggle("is-hidden", running);
    $("btn-stop").classList.toggle("is-hidden", !running);
    const badge = $("run-state-badge");
    badge.textContent = running ? "Sedang Berjalan..." : "Siap Mencari";
    badge.classList.toggle("is-running", running);
  }

  function applyStatusSnapshot(st) {
    if (!st) return;

    if (Array.isArray(st.logs) && st.logs.length) {
      st.logs.forEach(appendLog);
    }

    if (st.progress) {
      updateProgressBar("scrape", st.progress.scrape_done || 0, st.progress.scrape_target || 0);
      updateProgressBar("ai", st.progress.ai_done || 0, st.progress.ai_target || 0);
    }

    if (st.kpi) {
      $("kpi-total").textContent = String(st.kpi.total ?? 0);
      $("kpi-free").textContent = String(st.kpi.free ?? 0);
      $("kpi-paid").textContent = String(st.kpi.paid ?? 0);
      $("kpi-fast").textContent = String(st.kpi.fast ?? 0);
    }

    const aiPill = $("btn-ai-status");
    const aiText = $("ai-status-text");
    if (typeof st.ollama_active === "boolean" && aiPill && aiText) {
      aiPill.classList.toggle("is-ai-ready", st.ollama_active);
      aiText.textContent = st.ollama_active ? "● AI Aktif" : "○ Mode Regex";
    }

    if (typeof st.cache_count === "number") {
      $("cache-count").textContent = String(st.cache_count);
    }

    if (st.excel_path !== undefined) {
      state.excelPath = st.excel_path || "";
      $("btn-open-excel").disabled = !state.excelPath;
    }

    if (typeof st.results_version === "number" && st.results_version !== state.resultsVersion) {
      state.resultsVersion = st.results_version;
      state.results = Array.isArray(st.results) ? st.results : [];
      renderResultsTable(state.results);
    }

    updateRunningUI(!!st.running);
    if (!st.running && state.pollTimer) {
      clearInterval(state.pollTimer);
      state.pollTimer = null;
    }
  }

  async function pollStatusOnce() {
    if (!hasApi()) return;
    try {
      const st = await window.pywebview.api.get_status(state.logOffset);
      if (st && typeof st.next_log_offset === "number") {
        state.logOffset = st.next_log_offset;
      }
      applyStatusSnapshot(st);
    } catch (e) {}
  }

  async function startSearch(ev) {
    ev.preventDefault();
    if (!hasApi()) {
      showToast("Bridge Python WebView2 belum siap.");
      return;
    }
    const params = collectParams();
    if (!params.keyword) {
      showToast("Masukkan kata kunci terlebih dahulu.");
      $("input-keyword").focus();
      return;
    }
    if (!params.garuda && !params.sinta_src) {
      showToast("Pilih setidaknya satu sumber (Garuda atau SINTA).");
      return;
    }

    updateRunningUI(true);
    updateProgressBar("scrape", 0, params.target);
    updateProgressBar("ai", 0, 0);

    const res = await window.pywebview.api.start_scrape(params);
    if (res && res.ok === false) {
      updateRunningUI(false);
      showToast(res.message || "Gagal memulai pencarian.");
      return;
    }
    if (!state.pollTimer) {
      state.pollTimer = setInterval(pollStatusOnce, 250);
    }
  }

  async function refreshAiSetupDialog() {
    if (!hasApi()) return;
    const info = await window.pywebview.api.get_ai_setup_status();
    if (!info) return;
    $("ai-dlg-model").textContent = info.model || "-";
    $("ai-dlg-running").textContent = info.running ? "Aktif (localhost:11434)" : "Belum menyala";
    $("ai-dlg-installed").textContent = info.model_installed ? "Sudah terpasang" : "Belum diunduh";

    const wrap = $("ai-dlg-prog-wrap");
    if (info.task_running || info.task_message) {
      wrap.classList.remove("is-hidden");
      $("ai-dlg-prog-status").textContent = info.task_message || "Memproses...";
      const pct = Math.round(info.task_percent || 0);
      $("ai-dlg-prog-pct").textContent = `${pct}%`;
      $("ai-dlg-prog-fill").style.width = `${pct}%`;
    } else {
      wrap.classList.add("is-hidden");
    }

    if (!info.task_running && state.aiPollTimer) {
      clearInterval(state.aiPollTimer);
      state.aiPollTimer = null;
      pollStatusOnce();
    }
  }

  function startAiTaskPoll() {
    if (!state.aiPollTimer) {
      state.aiPollTimer = setInterval(refreshAiSetupDialog, 350);
    }
  }

  function bindEvents() {
    syncThemeUI();
    setupDialogs();

    $("btn-theme-toggle").addEventListener("click", toggleTheme);
    $("btn-guide").addEventListener("click", () => openDialog("dialog-guide"));
    $("btn-ai-status").addEventListener("click", () => {
      openDialog("dialog-ai-setup");
      refreshAiSetupDialog();
    });
    $("btn-open-ai-modal").addEventListener("click", () => {
      openDialog("dialog-ai-setup");
      refreshAiSetupDialog();
    });

    document.querySelectorAll(".preset-card").forEach((btn) => {
      btn.addEventListener("click", () => applyPreset(btn.getAttribute("data-preset"), false));
    });

    $("btn-months-all").addEventListener("click", () => setMonthChecks([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]));
    $("btn-months-none").addEventListener("click", () => setMonthChecks([]));
    $("btn-months-near").addEventListener("click", () => setMonthChecks(getNearThreeMonths()));

    $("search-form").addEventListener("submit", startSearch);
    $("btn-stop").addEventListener("click", async () => {
      if (hasApi()) {
        await window.pywebview.api.stop_scrape();
        showToast("Menghentikan proses...");
      }
    });

    $("btn-pick-manuscript").addEventListener("click", async () => {
      if (!hasApi()) return;
      const res = await window.pywebview.api.pick_manuscript();
      if (res && res.path) {
        state.manuscriptPath = res.path;
        $("manuscript-name").textContent = res.name || res.path;
        $("manuscript-banner").classList.remove("is-hidden");
        $("btn-clear-manuscript").classList.remove("is-hidden");
        if (res.suggested_keyword && !$("input-keyword").value.trim()) {
          $("input-keyword").value = res.suggested_keyword;
        }
        appendLog(`📎 Naskah dilampirkan: ${res.name || res.path}`);
      }
    });

    $("btn-clear-manuscript").addEventListener("click", () => {
      state.manuscriptPath = "";
      $("manuscript-banner").classList.add("is-hidden");
      $("btn-clear-manuscript").classList.add("is-hidden");
    });

    $("btn-clear-cache").addEventListener("click", async () => {
      if (!hasApi()) return;
      const res = await window.pywebview.api.clear_cache();
      showToast(res && res.message ? res.message : "Cache dibersihkan.");
      pollStatusOnce();
    });

    $("filter-search").addEventListener("input", applyTableFilters);
    $("filter-apc").addEventListener("change", applyTableFilters);

    document.querySelectorAll("#results-table thead th[data-sort-key]").forEach((th) => {
      const btn = th.querySelector(".th-sort");
      if (!btn) return;
      btn.addEventListener("click", () => {
        sortTableBy(th.dataset.sortKey, th.dataset.sortType, th);
      });
    });

    $("btn-copy-wa").addEventListener("click", async () => {
      if (!hasApi()) return;
      const visibleIndices = Array.from(document.querySelectorAll("#results-tbody tr:not(.empty-row):not(.is-hidden)"))
        .map((tr) => Number(tr.dataset.idx));
      const res = await window.pywebview.api.copy_share_list(visibleIndices);
      showToast(res && res.ok ? "Daftar format Share WA disalin ke clipboard!" : "Belum ada data untuk disalin.");
    });

    $("btn-copy-editor").addEventListener("click", async () => {
      if (!hasApi() || state.selectedIndex < 0) return;
      const res = await window.pywebview.api.copy_editor_chat(state.selectedIndex, $("input-keyword").value.trim());
      showToast(res && res.ok ? "Draf chat editor disalin ke clipboard!" : "Gagal menyalin draf chat.");
    });

    $("btn-detail-chat").addEventListener("click", async () => {
      if (!hasApi() || state.detailIndex < 0) return;
      const res = await window.pywebview.api.copy_editor_chat(state.detailIndex, $("input-keyword").value.trim());
      showToast(res && res.ok ? "Draf chat editor disalin ke clipboard!" : "Gagal menyalin draf chat.");
    });

    $("btn-detail-open-url").addEventListener("click", () => {
      const item = state.results[state.detailIndex];
      if (hasApi() && item && item.url) {
        window.pywebview.api.open_url(item.url);
      }
    });

    $("btn-open-excel").addEventListener("click", async () => {
      if (hasApi()) {
        const res = await window.pywebview.api.open_excel();
        if (res && res.ok === false) showToast(res.message || "File Excel belum tersedia.");
      }
    });

    $("btn-open-folder").addEventListener("click", async () => {
      if (hasApi()) await window.pywebview.api.open_output_folder();
    });

    // AI Modal Buttons
    $("btn-ai-refresh").addEventListener("click", refreshAiSetupDialog);
    $("btn-ai-launch").addEventListener("click", async () => {
      if (!hasApi()) return;
      const res = await window.pywebview.api.launch_ollama();
      showToast(res && res.message ? res.message : "Memeriksa layanan Ollama...");
      refreshAiSetupDialog();
    });
    $("btn-ai-install").addEventListener("click", async () => {
      if (!hasApi()) return;
      await window.pywebview.api.install_ollama();
      startAiTaskPoll();
    });
    $("btn-ai-pull").addEventListener("click", async () => {
      if (!hasApi()) return;
      await window.pywebview.api.pull_ai_model();
      startAiTaskPoll();
    });
  }

  function onBridgeReady() {
    applyPreset("skripsi", true);
    appendLog("✨ JSFS v2.0 siap digunakan (WebView2 Hardware-Accelerated Desktop UI).");
    pollStatusOnce();
    setTimeout(pollStatusOnce, 700);
  }

  document.addEventListener("DOMContentLoaded", () => {
    bindEvents();
    if (hasApi()) {
      onBridgeReady();
    } else {
      window.addEventListener("pywebviewready", onBridgeReady, { once: true });
    }
  });
})();


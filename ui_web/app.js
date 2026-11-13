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

  function clampTarget(val) {
    const num = Number(val);
    if (!Number.isFinite(num) || num < 1) return 25;
    return Math.min(1000, Math.round(num));
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
    if (text.includes("⚠")) line.classList.add("is-warn");
    else if (text.includes("❌") || text.includes("Error")) line.classList.add("is-err");
    else if (text.includes("✅") || text.includes("💾") || text.includes("Selesai")) line.classList.add("is-ok");
    line.textContent = text;
    box.appendChild(line);
    box.scrollTop = box.scrollHeight;
  }

  // ── Theme Toggle (Daylight Default + Dark Mode) ──────────────────────────────
  function applyScheme(scheme) {
    const clean = scheme === "dark" ? "dark" : "light";
    const meta = document.querySelector('meta[name="color-scheme"]');
    if (meta) meta.setAttribute("content", clean);
    document.documentElement.setAttribute("data-theme", clean);
  }

  function toggleTheme() {
    const meta = document.querySelector('meta[name="color-scheme"]');
    const cur = meta ? meta.getAttribute("content") : "light";
    const next = cur === "dark" ? "light" : "dark";
    applyScheme(next);
    try {
      localStorage.setItem("color-scheme", next);
    } catch (e) {}
  }

  // ── Native <dialog> Setup ────────────────────────────────────────────────────
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
    });

    if (!("closedBy" in HTMLDialogElement.prototype)) {
      const outside = (e, dialog) => {
        const r = dialog.getBoundingClientRect();
        return e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom;
      };
      document.querySelectorAll('dialog[closedby="any"]').forEach((dlg) => {
        let pressedOutside = false;
        dlg.addEventListener("pointerdown", (e) => {
          pressedOutside = outside(e, dlg);
        });
        dlg.addEventListener("pointerup", (e) => {
          if (pressedOutside && outside(e, dlg)) dlg.close();
        });
      });
    }

    document.querySelectorAll("[data-close-dialog]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const dlg = $(btn.getAttribute("data-close-dialog"));
        if (dlg && dlg.open) dlg.close();
      });
    });
  }

  // ── Target Sync (Main Input <-> Modal Input <-> Quick Chips) ─────────────────
  function setTargetCount(val) {
    const clamped = clampTarget(val);
    const mainInput = $("target-count");
    const modalInput = $("modal-target-count");
    if (mainInput) mainInput.value = String(clamped);
    if (modalInput) modalInput.value = String(clamped);
    if (clamped >= 100 && $("chk-turbo")) {
      $("chk-turbo").checked = true;
    }
    updateActiveFilterBadge();
    return clamped;
  }

  function updateActiveFilterBadge() {
    const sintaCount = document.querySelectorAll('input[name="sinta"]:checked').length;
    const monthCount = document.querySelectorAll('input[name="month"]:checked').length;
    const onlyFree = $("chk-only-free")?.checked;
    const useSel = $("chk-selenium")?.checked;
    const maxRp = parseCurrency($("input-max-rp")?.value);

    let activeCount = 0;
    if (sintaCount > 0 && sintaCount < 6) activeCount += 1;
    if (monthCount > 0 && monthCount < 12) activeCount += 1;
    if (onlyFree) activeCount += 1;
    if (maxRp > 0 && maxRp < 15000000) activeCount += 1;
    if (useSel) activeCount += 1;
    if (!$("chk-garuda")?.checked || !$("chk-sinta-src")?.checked) activeCount += 1;

    const badge = $("active-filter-badge");
    if (badge) {
      badge.textContent = String(activeCount);
      badge.classList.toggle("is-hidden", activeCount === 0);
    }
  }

  // ── Presets & Filter Helpers ─────────────────────────────────────────────────
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
      const active = card.getAttribute("data-preset") === key;
      card.classList.toggle("is-active", active);
      card.setAttribute("aria-pressed", active ? "true" : "false");
    });

    if (key === "skripsi") {
      setSintaChecks([3, 4, 5]);
      setTargetCount(25);
      $("chk-only-free").checked = false;
      $("chk-include-na").checked = true;
      $("input-min-rp").value = "0";
      $("input-max-rp").value = "1.500.000";
      setMonthChecks([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
      if (!silent) appendLog("Preset Aktif: Target Skripsi S1 (SINTA 3–5, maks Rp 1,5 jt).");
    } else if (key === "gratis") {
      setSintaChecks([2, 3, 4, 5, 6]);
      setTargetCount(50);
      $("chk-only-free").checked = true;
      $("chk-include-na").checked = true;
      setMonthChecks([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
      if (!silent) appendLog("Preset Aktif: Jurnal Gratis (Rp 0, SINTA 2–6).");
    } else if (key === "cepat") {
      setSintaChecks([3, 4, 5, 6]);
      setTargetCount(25);
      $("chk-only-free").checked = false;
      $("chk-include-na").checked = true;
      const near = getNearThreeMonths();
      setMonthChecks(near);
      const names = near.slice().sort((a, b) => a - b).map((m) => MONTH_NAMES[m - 1]).join(", ");
      if (!silent) appendLog(`Preset Aktif: Terbit Terdekat (${names}, SINTA 3–6).`);
    } else if (key === "reputasi") {
      setSintaChecks([1, 2]);
      setTargetCount(25);
      $("chk-only-free").checked = false;
      $("chk-include-na").checked = true;
      $("input-max-rp").value = "15.000.000";
      setMonthChecks([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
      if (!silent) appendLog("Preset Aktif: Reputasi Tinggi (SINTA 1–2).");
    }
    updateActiveFilterBadge();
  }

  function resetAllFilters() {
    $("chk-garuda").checked = true;
    $("chk-sinta-src").checked = true;
    $("chk-include-unknown-month").checked = true;
    $("chk-match").checked = true;
    $("chk-turbo").checked = true;
    $("chk-selenium").checked = false;
    $("chk-ai").checked = true;
    $("select-ai-workers").value = "2";
    document.querySelectorAll('input[name="fmt"]').forEach((cb) => {
      cb.checked = cb.value === ".xlsx" || cb.value === ".md";
    });
    applyPreset("skripsi", false);
    showToast("Filter dikembalikan ke pengaturan default.");
  }

  function parseCurrency(raw) {
    const digits = String(raw || "").replace(/\D+/g, "");
    return digits ? Number(digits) : 0;
  }

  function collectParams() {
    const sinta = Array.from(document.querySelectorAll('input[name="sinta"]:checked')).map((c) => Number(c.value));
    const months = Array.from(document.querySelectorAll('input[name="month"]:checked')).map((c) => Number(c.value));
    const formats = Array.from(document.querySelectorAll('input[name="fmt"]:checked')).map((c) => c.value);
    const target = clampTarget($("target-count").value);

    return {
      keyword: $("input-keyword").value.trim(),
      manuscript: state.manuscriptPath,
      sinta: sinta.length ? sinta : [1, 2, 3, 4, 5, 6],
      months: months.length ? months : [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
      include_unknown: $("chk-include-unknown-month").checked,
      min_rp: parseCurrency($("input-min-rp").value),
      max_rp: parseCurrency($("input-max-rp").value) || 15000000,
      target,
      garuda: $("chk-garuda").checked,
      sinta_src: $("chk-sinta-src").checked,
      only_free: $("chk-only-free").checked,
      include_na: $("chk-include-na").checked,
      match: $("chk-match").checked,
      turbo: $("chk-turbo").checked || target >= 100,
      selenium: $("chk-selenium").checked,
      ai_enabled: $("chk-ai").checked,
      ai_workers: Number($("select-ai-workers").value) || 2,
      formats: formats.length ? formats : [".xlsx", ".md"],
    };
  }

  // ── Results Rendering & Sortable Table ───────────────────────────────────────
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
    const emptyPanel = $("empty-state-panel");
    const tableWrap = $("results-table-wrap");
    if (!tbody || !emptyPanel || !tableWrap) return;

    tbody.innerHTML = "";
    state.selectedIndex = -1;
    $("btn-copy-editor").disabled = true;

    const hasItems = Array.isArray(items) && items.length > 0;
    emptyPanel.classList.toggle("is-hidden", hasItems);
    tableWrap.classList.toggle("is-hidden", !hasItems);
    $("btn-copy-wa").disabled = !hasItems;

    if (!hasItems) {
      updateVisibleRowNumbers();
      return;
    }

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

      const tdNum = document.createElement("td");
      tdNum.className = "col-num";
      tdNum.textContent = String(idx + 1);

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
        warn.textContent = "Data kontak/jadwal minim — cek situs";
        tdName.appendChild(warn);
      }

      const tdSinta = document.createElement("td");
      const badgeSinta = document.createElement("span");
      badgeSinta.className = "badge badge-sinta";
      badgeSinta.textContent = item.sinta ? `SINTA ${item.sinta}` : "Garuda";
      tdSinta.appendChild(badgeSinta);

      const tdMatch = document.createElement("td");
      const scoreEl = document.createElement("div");
      scoreEl.className = "match-score";
      scoreEl.textContent = `${item.match_score ?? 40}%`;
      const levelEl = document.createElement("div");
      levelEl.className = "muted";
      levelEl.textContent = item.kesesuaian_naskah || "Cukup Sesuai";
      tdMatch.append(scoreEl, levelEl);

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

      const tdJadwal = document.createElement("td");
      const jadTop = document.createElement("div");
      jadTop.textContent = item.jadwal_terbit || "-";
      const jadEst = document.createElement("div");
      jadEst.className = "muted";
      jadEst.textContent = `Terdekat: ${item.terbit_terdekat || "-"}`;
      tdJadwal.append(jadTop, jadEst);

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
      btnDetail.textContent = "Detail";
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
        btnWeb.textContent = "Buka Web";
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
    document.querySelectorAll("#results-tbody tr").forEach((tr) => {
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

    document.querySelectorAll("#results-tbody tr").forEach((tr) => {
      const matchText = !q || (tr.dataset.search || "").includes(q);
      const matchApc = apc === "all" || tr.dataset.apc === apc;
      tr.classList.toggle("is-hidden", !(matchText && matchApc));
    });
    updateVisibleRowNumbers();
  }

  function sortTableBy(colKey, sortType, thEl, labelText) {
    const tbody = $("results-tbody");
    if (!tbody || !state.results.length) return;

    const nextDir = thEl.getAttribute("aria-sort") === "ascending" ? "descending" : "ascending";
    document.querySelectorAll("#results-table thead th").forEach((th) => {
      if (th === thEl) th.setAttribute("aria-sort", nextDir);
      else th.removeAttribute("aria-sort");
    });

    const dataKey = "sort" + colKey.charAt(0).toUpperCase() + colKey.slice(1);
    const rows = Array.from(tbody.querySelectorAll("tr"));
    rows.sort((a, b) => {
      const va = a.dataset[dataKey] ?? "";
      const vb = b.dataset[dataKey] ?? "";
      const cmp =
        sortType === "number"
          ? (Number(va) || 0) - (Number(vb) || 0)
          : collator.compare(va, vb);
      return nextDir === "ascending" ? cmp : -cmp;
    });

    rows.forEach((row) => {
      if (typeof tbody.moveBefore === "function") tbody.moveBefore(row, null);
      else tbody.appendChild(row);
    });

    updateVisibleRowNumbers();
    const status = $("sort-status");
    if (status) {
      status.textContent = `Diurutkan berdasarkan ${labelText}, ${nextDir === "ascending" ? "menaik" : "menurun"}, ${rows.length} baris`;
    }
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
      ["Tautan Situs / Halaman Biaya", item.apc_url || item.url || "-", true],
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

  // ── Backend Sync & Polling ───────────────────────────────────────────────────
  function updateProgressBar(prefix, done, total) {
    const pct = total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 0;
    $(`${prefix}-prog-text`).textContent = `${done} / ${total}`;
    $(`${prefix}-prog-fill`).style.width = `${pct}%`;
    const wrap = $(`${prefix}-prog-bar-wrap`);
    if (wrap) wrap.setAttribute("aria-valuenow", String(pct));
  }

  function updateRunningUI(running, phaseStatus) {
    state.running = running;
    $("btn-start").classList.toggle("is-hidden", running);
    $("btn-stop").classList.toggle("is-hidden", !running);
    const badge = $("run-state-badge");
    if (badge) {
      badge.textContent = phaseStatus || (running ? "Sedang mencari..." : "Siap mencari");
      badge.classList.toggle("is-running", running);
    }
    if (running && $("scrape-label")) {
      $("scrape-label").textContent = "Mencari jurnal dari SINTA & Garuda...";
      $("ai-label").textContent = "Memeriksa biaya & jadwal terbit...";
    } else if (!running && state.results.length > 0) {
      $("scrape-label").textContent = "Pencarian portal selesai";
      $("ai-label").textContent = "Pemeriksaan biaya & jadwal selesai";
    }
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
      aiText.textContent = st.ollama_active ? "AI Ollama Aktif" : "Mode Regex Cepat";
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

    updateRunningUI(!!st.running, st.phase_status);
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
    if (ev) ev.preventDefault();
    if (!hasApi()) {
      showToast("Menunggu sambungan desktop selesai dimuat...");
      return;
    }
    const params = collectParams();
    if (!params.keyword) {
      showToast("Masukkan topik atau kata kunci penelitian terlebih dahulu.");
      $("input-keyword").focus();
      return;
    }
    if (!params.garuda && !params.sinta_src) {
      showToast("Aktifkan minimal satu portal (Garuda atau SINTA) di Filter Lanjutan.");
      return;
    }

    setTargetCount(params.target);
    updateRunningUI(true, "Mencari jurnal dari SINTA & Garuda...");
    updateProgressBar("scrape", 0, params.target);
    updateProgressBar("ai", 0, 0);

    const apiMethod = window.pywebview.api.start_search || window.pywebview.api.start_scrape;
    const res = await apiMethod(params);
    if (res && res.ok === false) {
      updateRunningUI(false);
      showToast(res.message || "Gagal memulai pencarian.");
      return;
    }
    if (!state.pollTimer) {
      state.pollTimer = setInterval(pollStatusOnce, 250);
    }
  }

  async function uploadManuscript() {
    if (!hasApi()) return;
    const apiMethod = window.pywebview.api.upload_manuscript || window.pywebview.api.pick_manuscript;
    const res = await apiMethod();
    if (res && res.path) {
      state.manuscriptPath = res.path;
      $("manuscript-name").textContent = res.name || res.path;
      $("manuscript-banner").classList.remove("is-hidden");
      $("btn-clear-manuscript").classList.remove("is-hidden");
      if (res.suggested_keyword && !$("input-keyword").value.trim()) {
        $("input-keyword").value = res.suggested_keyword;
      }
      appendLog(`Naskah dilampirkan: ${res.name || res.path}`);
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

  function bindEvents() {
    setupDialogs();

    window.addEventListener("storage", (e) => {
      if (e.key === "color-scheme") {
        applyScheme(e.newValue || "light");
      }
    });

    $("btn-theme-toggle").addEventListener("click", toggleTheme);
    $("btn-guide").addEventListener("click", () => openDialog("dialog-guide"));
    $("btn-advanced-filter").addEventListener("click", () => openDialog("dialog-filters"));

    $("btn-apply-filters").addEventListener("click", () => {
      setTargetCount($("modal-target-count").value);
      updateActiveFilterBadge();
      const dlg = $("dialog-filters");
      if (dlg && dlg.open) dlg.close();
      showToast("Filter pencarian diterapkan.");
    });

    $("btn-reset-filters").addEventListener("click", resetAllFilters);

    $("btn-ai-status").addEventListener("click", () => {
      openDialog("dialog-ai-setup");
      refreshAiSetupDialog();
    });

    $("btn-open-ai-modal").addEventListener("click", () => {
      const fd = $("dialog-filters");
      if (fd && fd.open) fd.close();
      openDialog("dialog-ai-setup");
      refreshAiSetupDialog();
    });

    $("target-count").addEventListener("input", (e) => {
      if ($("modal-target-count")) $("modal-target-count").value = e.target.value;
    });
    $("target-count").addEventListener("change", (e) => setTargetCount(e.target.value));

    $("modal-target-count").addEventListener("input", (e) => {
      if ($("target-count")) $("target-count").value = e.target.value;
    });
    $("modal-target-count").addEventListener("change", (e) => setTargetCount(e.target.value));

    document.querySelectorAll(".chip-btn[data-target-val]").forEach((btn) => {
      btn.addEventListener("click", () => {
        setTargetCount(btn.getAttribute("data-target-val"));
      });
    });

    document.querySelectorAll("#dialog-filters input, #dialog-filters select").forEach((el) => {
      el.addEventListener("change", updateActiveFilterBadge);
    });

    document.querySelectorAll(".preset-card").forEach((btn) => {
      btn.addEventListener("click", () => applyPreset(btn.getAttribute("data-preset"), false));
    });

    $("btn-months-all").addEventListener("click", () => {
      setMonthChecks([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
      updateActiveFilterBadge();
    });
    $("btn-months-none").addEventListener("click", () => {
      setMonthChecks([]);
      updateActiveFilterBadge();
    });
    $("btn-months-near").addEventListener("click", () => {
      setMonthChecks(getNearThreeMonths());
      updateActiveFilterBadge();
    });

    $("search-form").addEventListener("submit", startSearch);
    $("btn-stop").addEventListener("click", async () => {
      if (hasApi()) {
        await window.pywebview.api.stop_scrape();
        showToast("Menghentikan pencarian...");
      }
    });

    $("btn-pick-manuscript").addEventListener("click", uploadManuscript);
    $("btn-clear-manuscript").addEventListener("click", () => {
      state.manuscriptPath = "";
      $("manuscript-banner").classList.add("is-hidden");
      $("btn-clear-manuscript").classList.add("is-hidden");
    });

    $("btn-toggle-log").addEventListener("click", () => {
      const drawer = $("technical-log-drawer");
      const hidden = drawer.classList.toggle("is-hidden");
      $("btn-toggle-log").setAttribute("aria-expanded", hidden ? "false" : "true");
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
        sortTableBy(th.dataset.sortKey, th.dataset.sortType, th, btn.textContent.trim());
      });
    });

    $("btn-copy-wa").addEventListener("click", async () => {
      if (!hasApi()) return;
      const visibleIndices = Array.from(document.querySelectorAll("#results-tbody tr:not(.is-hidden)"))
        .map((tr) => Number(tr.dataset.idx));
      const res = await window.pywebview.api.copy_share_list(visibleIndices);
      showToast(res && res.ok ? "Daftar format Share WA disalin ke clipboard!" : "Belum ada data untuk disalin.");
    });

    const copyEditorHandler = async (idx) => {
      if (!hasApi() || idx < 0) return;
      const fn = window.pywebview.api.copy_editor_draft || window.pywebview.api.copy_editor_chat;
      const res = await fn(idx, $("input-keyword").value.trim());
      showToast(res && res.ok ? "Draf pesan editor disalin ke clipboard!" : "Gagal menyalin draf pesan.");
    };

    $("btn-copy-editor").addEventListener("click", () => copyEditorHandler(state.selectedIndex));
    $("btn-detail-chat").addEventListener("click", () => copyEditorHandler(state.detailIndex));

    $("btn-detail-open-url").addEventListener("click", () => {
      const item = state.results[state.detailIndex];
      if (hasApi() && item && item.url) window.pywebview.api.open_url(item.url);
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
      if (!state.aiPollTimer) state.aiPollTimer = setInterval(refreshAiSetupDialog, 350);
    });
    $("btn-ai-pull").addEventListener("click", async () => {
      if (!hasApi()) return;
      await window.pywebview.api.pull_ai_model();
      if (!state.aiPollTimer) state.aiPollTimer = setInterval(refreshAiSetupDialog, 350);
    });
  }

  function onBridgeReady() {
    applyPreset("skripsi", true);
    renderResultsTable([]);
    appendLog("JSFS v2.0.1 siap digunakan.");
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

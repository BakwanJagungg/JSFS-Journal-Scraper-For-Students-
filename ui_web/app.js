(function () {
  "use strict";

  const MONTH_NAMES = ["Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des"];
  const collator = new Intl.Collator("id", { numeric: true, sensitivity: "base" });

  const state = {
    manuscriptPath: "",
    running: false,
    results: [],
    resultsVersion: -1,
    filteredIndices: [],
    currentPage: 1,
    pageSize: 50,
    sortKey: "",
    sortType: "text",
    sortDir: "",
    lastTableRenderMs: 0,
    pendingTableRefresh: false,
    selectedIndex: -1,
    excelPath: "",
    activePreset: "skripsi",
    pollTimer: null,
    clockTimer: null,
    runStartedClientMs: 0,
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

  function _itemSearchHaystack(item) {
    if (typeof item._searchHaystack === "string") return item._searchHaystack;
    item._searchHaystack = [
      item.nama,
      item.penerbit,
      item.focus_scope,
      item.email,
      item.telepon,
      item.apc_label,
      item.jadwal_terbit,
    ].join(" ").toLowerCase();
    return item._searchHaystack;
  }

  function _getSortValue(item, key) {
    if (key === "nama") return String(item.nama || "");
    if (key === "sinta") return Number(item.sinta) || 99;
    if (key === "match_score") return Number(item.match_score ?? 0);
    if (key === "biaya_sort") return Number(item.biaya_sort ?? 999999999);
    if (key === "jadwal_sort") return String(item.terbit_terdekat || "zzz");
    return "";
  }

  function computeFilteredSortedIndices() {
    const qEl = $("filter-search");
    const apcEl = $("filter-apc");
    const q = (qEl ? qEl.value : "").trim().toLowerCase();
    const apc = apcEl ? apcEl.value : "all";

    const indices = [];
    for (let i = 0; i < state.results.length; i += 1) {
      const item = state.results[i];
      if (!item) continue;
      if (apc !== "all" && classifyApc(item) !== apc) continue;
      if (q && !_itemSearchHaystack(item).includes(q)) continue;
      indices.push(i);
    }

    if (state.sortKey && state.sortDir) {
      const key = state.sortKey;
      const isNum = state.sortType === "number";
      const dirMult = state.sortDir === "ascending" ? 1 : -1;
      indices.sort((ia, ib) => {
        const va = _getSortValue(state.results[ia], key);
        const vb = _getSortValue(state.results[ib], key);
        const cmp = isNum ? (Number(va) - Number(vb)) : collator.compare(String(va), String(vb));
        return cmp * dirMult;
      });
    }

    state.filteredIndices = indices;
    return indices;
  }

  function renderResultsTable(items) {
    if (Array.isArray(items) && items !== state.results) {
      state.results = items;
    }

    const tbody = $("results-tbody");
    const emptyPanel = $("empty-state-panel");
    const tableWrap = $("results-table-wrap");
    const pagBar = $("pagination-bar");
    if (!tbody || !emptyPanel || !tableWrap) return;

    const totalAll = state.results.length;
    const hasAnyResults = totalAll > 0;
    emptyPanel.classList.toggle("is-hidden", hasAnyResults);
    tableWrap.classList.toggle("is-hidden", !hasAnyResults);
    if (pagBar) pagBar.classList.toggle("is-hidden", !hasAnyResults);

    if (!hasAnyResults) {
      tbody.textContent = "";
      state.filteredIndices = [];
      state.selectedIndex = -1;
      $("btn-copy-editor").disabled = true;
      $("btn-copy-wa").disabled = true;
      const pill = $("results-count-pill");
      if (pill) pill.textContent = "0 jurnal";
      if ($("pagination-info")) $("pagination-info").textContent = "Halaman 1 dari 1 (0 jurnal)";
      return;
    }

    const filtered = computeFilteredSortedIndices();
    const totalFiltered = filtered.length;
    $("btn-copy-wa").disabled = totalFiltered === 0;

    const pill = $("results-count-pill");
    if (pill) {
      pill.textContent = totalFiltered === totalAll ? `${totalAll} jurnal` : `${totalFiltered} dari ${totalAll} jurnal`;
    }

    const perPage = state.pageSize === "all" ? Math.max(1, totalFiltered) : Math.max(1, Number(state.pageSize) || 50);
    const totalPages = Math.max(1, Math.ceil(totalFiltered / perPage));
    if (state.currentPage > totalPages) state.currentPage = totalPages;
    if (state.currentPage < 1) state.currentPage = 1;

    const startOffset = (state.currentPage - 1) * perPage;
    const pageSlice = filtered.slice(startOffset, startOffset + perPage);

    if (state.selectedIndex >= 0 && !pageSlice.includes(state.selectedIndex)) {
      state.selectedIndex = -1;
      $("btn-copy-editor").disabled = true;
    }

    if ($("pagination-info")) {
      $("pagination-info").textContent = `Halaman ${state.currentPage} dari ${totalPages} (${totalFiltered} jurnal)`;
    }
    if ($("btn-page-prev")) $("btn-page-prev").disabled = state.currentPage <= 1;
    if ($("btn-page-next")) $("btn-page-next").disabled = state.currentPage >= totalPages;

    const frag = document.createDocumentFragment();
    pageSlice.forEach((origIdx, slicePos) => {
      const item = state.results[origIdx];
      const tr = document.createElement("tr");
      const apcCat = classifyApc(item);
      tr.dataset.idx = String(origIdx);
      tr.dataset.apc = apcCat;
      if (origIdx === state.selectedIndex) tr.classList.add("is-selected");

      const tdNum = document.createElement("td");
      tdNum.className = "col-num";
      tdNum.textContent = String(startOffset + slicePos + 1);

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
        selectRow(origIdx);
        openJournalDetail(origIdx);
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
      tr.addEventListener("click", () => selectRow(origIdx));
      frag.appendChild(tr);
    });

    tbody.textContent = "";
    tbody.appendChild(frag);
    state.lastTableRenderMs = Date.now();
    state.pendingTableRefresh = false;
  }

  function selectRow(idx) {
    state.selectedIndex = idx;
    document.querySelectorAll("#results-tbody tr").forEach((tr) => {
      tr.classList.toggle("is-selected", Number(tr.dataset.idx) === idx);
    });
    $("btn-copy-editor").disabled = idx < 0;
  }

  function applyTableFilters() {
    state.currentPage = 1;
    renderResultsTable();
  }

  function sortTableBy(colKey, sortType, thEl, labelText) {
    if (!state.results.length) return;

    const nextDir = thEl.getAttribute("aria-sort") === "ascending" ? "descending" : "ascending";
    document.querySelectorAll("#results-table thead th").forEach((th) => {
      if (th === thEl) th.setAttribute("aria-sort", nextDir);
      else th.removeAttribute("aria-sort");
    });

    state.sortKey = colKey;
    state.sortType = sortType || "text";
    state.sortDir = nextDir;
    state.currentPage = 1;
    renderResultsTable();

    const status = $("sort-status");
    if (status) {
      status.textContent = `Diurutkan berdasarkan ${labelText}, ${nextDir === "ascending" ? "menaik" : "menurun"}, ${state.filteredIndices.length} baris`;
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
  function formatElapsed(sec) {
    const total = Math.max(0, Math.floor(Number(sec) || 0));
    const mm = String(Math.floor(total / 60)).padStart(2, "0");
    const ss = String(total % 60).padStart(2, "0");
    return `${mm}:${ss}`;
  }

  function playCompletionChime() {
    if (localStorage.getItem("jsfs-sound-alert") === "off") return;
    try {
      const AudioCtx = window.AudioContext || window.webkitAudioContext;
      if (!AudioCtx) return;
      const ctx = new AudioCtx();
      const now = ctx.currentTime;
      const notes = [523.25, 659.25];
      notes.forEach((freq, idx) => {
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = "sine";
        osc.frequency.setValueAtTime(freq, now + idx * 0.14);
        gain.gain.setValueAtTime(0.0001, now + idx * 0.14);
        gain.gain.exponentialRampToValueAtTime(0.055, now + idx * 0.14 + 0.02);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + idx * 0.14 + 0.24);
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.start(now + idx * 0.14);
        osc.stop(now + idx * 0.14 + 0.25);
      });
      setTimeout(() => {
        try { ctx.close(); } catch (e) {}
      }, 900);
    } catch (e) {}
  }

  function applySoundPreference() {
    const enabled = localStorage.getItem("jsfs-sound-alert") !== "off";
    const btn = $("btn-sound-toggle");
    const lbl = $("sound-toggle-label");
    if (btn) btn.setAttribute("aria-pressed", enabled ? "true" : "false");
    if (lbl) lbl.textContent = enabled ? "Suara On" : "Suara Off";
  }

  function showCompletionBanner(durationSec, totalCount) {
    const banner = $("completion-banner");
    const textEl = $("completion-banner-text");
    const msg = `Pencarian selesai dalam ${formatElapsed(durationSec)} — ${totalCount} jurnal valid ditemukan`;
    if (textEl) textEl.textContent = msg;
    if (banner) banner.classList.remove("is-hidden");
    showToast(msg);
  }

  function updateProgressBar(prefix, done, total, isIndeterminate) {
    const bar = $(`${prefix}-prog-bar`);
    const txt = $(`${prefix}-prog-text`);
    const numDone = Number(done) || 0;
    const numTotal = Number(total) || 0;

    if (isIndeterminate) {
      if (bar) {
        bar.removeAttribute("value");
        bar.classList.add("indeterminate");
      }
      if (txt) txt.textContent = numDone > 0 ? `${numDone} hal.` : "Memindai...";
      return;
    }

    if (bar) {
      bar.classList.remove("indeterminate");
      const safeMax = Math.max(1, numTotal);
      bar.max = safeMax;
      bar.value = Math.min(safeMax, Math.max(0, numDone));
    }
    if (txt) txt.textContent = `${numDone} / ${numTotal}`;
  }

  function updateRunningUI(running, phaseStatus, phase) {
    state.running = running;
    $("btn-start").classList.toggle("is-hidden", running);
    $("btn-stop").classList.toggle("is-hidden", !running);
    const badge = $("run-state-badge");
    if (badge) {
      badge.textContent = phaseStatus || (running ? "Sedang mencari..." : "Siap mencari");
      badge.classList.toggle("is-running", running);
    }

    const scrapeLbl = $("scrape-label");
    const aiLbl = $("ai-label");
    if (running) {
      if (phase === "fase3_ai" || phase === "fase4_export") {
        if (scrapeLbl) scrapeLbl.textContent = "Pencarian & deep crawl selesai";
        if (aiLbl) aiLbl.textContent = phaseStatus || "Validasi cerdas AI & ekspor...";
      } else {
        if (scrapeLbl) scrapeLbl.textContent = phaseStatus || "Menjelajahi halaman portal SINTA & Garuda...";
        if (aiLbl) aiLbl.textContent = "Menunggu antrean validasi biaya & jadwal";
      }
    } else if (state.results.length > 0) {
      if (scrapeLbl) scrapeLbl.textContent = "Pencarian portal selesai";
      if (aiLbl) aiLbl.textContent = "Pemeriksaan biaya & jadwal selesai";
    } else {
      if (scrapeLbl) scrapeLbl.textContent = "Siap menjelajahi SINTA & Garuda";
      if (aiLbl) aiLbl.textContent = "Memeriksa biaya & jadwal terbit";
    }
  }

  function applyStatusSnapshot(st) {
    if (!st) return;

    if (Array.isArray(st.logs) && st.logs.length) {
      st.logs.forEach(appendLog);
    }

    const prog = st.progress || {};
    const sDone = st.scrape_done ?? prog.scrape_done ?? 0;
    const sTotal = st.scrape_total ?? prog.scrape_target ?? 0;
    const aDone = st.ai_done ?? prog.ai_done ?? 0;
    const aTotal = st.ai_total ?? prog.ai_target ?? 0;
    const phase = st.phase || "idle";
    const isPortalIndeterminate = !!st.running && (sTotal === 0) && (phase === "fase1_portal" || phase === "idle");

    updateProgressBar("scrape", sDone, sTotal, isPortalIndeterminate);
    updateProgressBar("ai", aDone, aTotal, !!st.running && phase === "fase3_ai" && aTotal === 0);

    const kpiData = st.live_counts || st.kpi;
    if (kpiData) {
      $("kpi-total").textContent = String(kpiData.total ?? 0);
      $("kpi-free").textContent = String(kpiData.free ?? 0);
      $("kpi-paid").textContent = String(kpiData.paid ?? 0);
      $("kpi-fast").textContent = String(kpiData.fast ?? 0);
    }

    if ($("elapsed-timer") && typeof st.elapsed_seconds === "number") {
      $("elapsed-timer").textContent = formatElapsed(st.elapsed_seconds);
    }
    if ($("scrape-rate") && typeof st.rate_per_sec === "number") {
      $("scrape-rate").textContent = `~${st.rate_per_sec.toFixed(1)}/dtk`;
    }
    const ticker = $("current-item-ticker");
    if (ticker) {
      if (st.running && st.current_item) {
        ticker.textContent = st.current_item;
        ticker.title = st.current_item;
      } else if (!st.running && state.results.length > 0) {
        ticker.textContent = `Selesai: ${state.results.length} jurnal siap ditinjau di tabel.`;
      } else if (!st.running) {
        ticker.textContent = "Masukkan topik lalu klik Cari Jurnal untuk memulai";
      }
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

    let hasNewResults = false;
    if (typeof st.results_version === "number" && st.results_version !== state.resultsVersion) {
      state.resultsVersion = st.results_version;
      if (Array.isArray(st.results)) {
        state.results = st.results;
        hasNewResults = true;
      }
    }

    if (st.running) {
      if (hasNewResults) state.pendingTableRefresh = true;
      if (state.pendingTableRefresh && Date.now() - state.lastTableRenderMs >= 1500) {
        renderResultsTable();
      }
    } else if (hasNewResults || state.pendingTableRefresh) {
      state.currentPage = 1;
      renderResultsTable();
    }

    const wasRunning = state.running;
    updateRunningUI(!!st.running, st.phase_label || st.phase_status, phase);
    if (!st.running) {
      if (state.pollTimer) {
        clearInterval(state.pollTimer);
        state.pollTimer = null;
      }
      if (state.clockTimer) {
        clearInterval(state.clockTimer);
        state.clockTimer = null;
      }
      if (wasRunning && (phase === "done" || state.results.length > 0)) {
        const durSec = typeof st.elapsed_seconds === "number"
          ? st.elapsed_seconds
          : Math.max(0, (Date.now() - (state.runStartedClientMs || Date.now())) / 1000);
        showCompletionBanner(durSec, state.results.length);
        playCompletionChime();
      }
    }
  }

  async function pollStatusOnce() {
    if (!hasApi()) return;
    try {
      const st = await window.pywebview.api.get_status(state.logOffset, state.resultsVersion);
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
    state.currentPage = 1;
    state.pendingTableRefresh = false;
    state.lastTableRenderMs = 0;
    state.runStartedClientMs = Date.now();
    if ($("completion-banner")) $("completion-banner").classList.add("is-hidden");
    if ($("elapsed-timer")) $("elapsed-timer").textContent = "00:00";
    if ($("scrape-rate")) $("scrape-rate").textContent = "~0.0/dtk";
    if ($("current-item-ticker")) $("current-item-ticker").textContent = `Menghubungi portal untuk '${params.keyword}'...`;
    updateRunningUI(true, "Menjelajahi halaman portal SINTA & Garuda...", "fase1_portal");
    updateProgressBar("scrape", 0, 0, true);
    updateProgressBar("ai", 0, 0, false);

    const apiMethod = window.pywebview.api.start_search || window.pywebview.api.start_scrape;
    const res = await apiMethod(params);
    if (res && res.ok === false) {
      updateRunningUI(false);
      showToast(res.message || "Gagal memulai pencarian.");
      return;
    }
    if (!state.pollTimer) {
      state.pollTimer = setInterval(pollStatusOnce, 150);
    }
    if (!state.clockTimer) {
      state.clockTimer = setInterval(() => {
        if (state.running && $("elapsed-timer") && state.runStartedClientMs > 0) {
          $("elapsed-timer").textContent = formatElapsed((Date.now() - state.runStartedClientMs) / 1000);
        }
      }, 500);
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
      const visibleIndices = Array.isArray(state.filteredIndices) ? state.filteredIndices : [];
      const res = await window.pywebview.api.copy_share_list(visibleIndices);
      showToast(res && res.ok ? "Daftar format Share WA disalin ke clipboard!" : "Belum ada data untuk disalin.");
    });

    if ($("btn-page-prev")) {
      $("btn-page-prev").addEventListener("click", () => {
        if (state.currentPage > 1) {
          state.currentPage -= 1;
          renderResultsTable();
        }
      });
    }
    if ($("btn-page-next")) {
      $("btn-page-next").addEventListener("click", () => {
        state.currentPage += 1;
        renderResultsTable();
      });
    }
    if ($("select-page-size")) {
      $("select-page-size").addEventListener("change", (e) => {
        const val = e.target.value;
        state.pageSize = val === "all" ? "all" : Math.max(10, Number(val) || 50);
        state.currentPage = 1;
        renderResultsTable();
      });
    }

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

    const btnOpenLogs = $("btn-open-logs");
    if (btnOpenLogs) {
      btnOpenLogs.addEventListener("click", async () => {
        if (hasApi() && typeof window.pywebview.api.open_logs_folder === "function") {
          await window.pywebview.api.open_logs_folder();
        }
      });
    }

    if ($("btn-close-completion")) {
      $("btn-close-completion").addEventListener("click", () => {
        if ($("completion-banner")) $("completion-banner").classList.add("is-hidden");
      });
    }

    if ($("btn-sound-toggle")) {
      $("btn-sound-toggle").addEventListener("click", () => {
        const currentlyOn = localStorage.getItem("jsfs-sound-alert") !== "off";
        localStorage.setItem("jsfs-sound-alert", currentlyOn ? "off" : "on");
        applySoundPreference();
        showToast(currentlyOn ? "Suara notifikasi dimatikan." : "Suara notifikasi diaktifkan.");
      });
    }

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
    applySoundPreference();
    renderResultsTable([]);
    appendLog("JSFS v2.2.0 siap digunakan.");
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

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
    heartbeatTimer: null,
    lifecycleBound: false,
    runStartedClientMs: 0,
    latestEtaLabel: "",
    aiPollTimer: null,
    logOffset: 0,
    detailIndex: -1,
    clearCloudKeyPending: false,
  };

  function $(id) {
    return document.getElementById(id);
  }

  function getTauriCore() {
    return window.__TAURI__ && window.__TAURI__.core ? window.__TAURI__.core : null;
  }

  function getTauriEvent() {
    return window.__TAURI__ && window.__TAURI__.event ? window.__TAURI__.event : null;
  }

  function hasApi() {
    const core = getTauriCore();
    return !!(core && typeof core.invoke === "function");
  }

  async function writeClipboardText(text) {
    if (!text) return false;
    try {
      if (navigator.clipboard && typeof navigator.clipboard.writeText === "function") {
        await navigator.clipboard.writeText(text);
        return true;
      }
    } catch (e) {}
    try {
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      const ok = document.execCommand("copy");
      document.body.removeChild(ta);
      return ok;
    } catch (e) {
      return false;
    }
  }

  async function callBackendApi(action, payload) {
    const core = getTauriCore();
    if (!core || typeof core.invoke !== "function") return null;
    const invoke = core.invoke.bind(core);

    switch (action) {
      case "init":
      case "aiStatus":
        return await invoke("get_ai_status");
      case "search":
        return await invoke("start_search", { params: payload });
      case "stop":
        return await invoke("stop_search");
      case "uploadManuscript":
        return await invoke("upload_manuscript", payload || {});
      case "copyShare": {
        const res = await invoke("copy_share_list", { indices: Array.isArray(payload) ? payload : null });
        if (res && res.ok && res.text) await writeClipboardText(res.text);
        return res;
      }
      case "copyDraft": {
        const res = await invoke("copy_editor_draft", {
          index: Number(payload?.index ?? 0),
          keyword: String(payload?.keyword ?? ""),
        });
        if (res && res.ok && res.text) await writeClipboardText(res.text);
        return res;
      }
      case "openExcel":
        return await invoke("open_excel", { path: state.excelPath || null });
      case "openOutput":
        return await invoke("open_output_folder");
      case "openLogs":
        return await invoke("open_logs_folder");
      case "openUrl":
        return await invoke("open_url", { url: String(payload || "") });
      case "clearCache":
        return await invoke("clear_cache");
      case "launchOllama":
        return await invoke("launch_ollama");
      case "installOllama":
        return await invoke("install_ollama");
      case "pullAiModel":
        return await invoke("pull_ai_model", { model: payload?.model ?? null });
      case "saveMatcherConfig":
        return await invoke("save_matcher_config", { input: payload });
      case "testCloudApi":
        return await invoke("test_cloud_api", { input: payload });
      default:
        return null;
    }
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
          if (hasApi()) callBackendApi("openUrl", item.url);
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

  function formatTimerText(sec, etaLabel) {
    const base = formatElapsed(sec);
    return etaLabel ? `${base} • ${etaLabel}` : base;
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

  function defaultCloudModelFor(provider) {
    if (provider === "gemini") return "gemini-2.5-flash";
    if (provider === "openai_compatible") return "gpt-4o-mini";
    return "llama-3.1-8b-instant";
  }

  function syncMatcherSubpanels(modeVal, providerVal) {
    const mode = modeVal || document.querySelector('input[name="matcher-mode"]:checked')?.value || "builtin";
    const prov = providerVal || $("select-cloud-provider")?.value || "groq";
    if ($("matcher-panel-cloud")) $("matcher-panel-cloud").classList.toggle("is-hidden", mode !== "cloud");
    if ($("matcher-panel-ollama")) $("matcher-panel-ollama").classList.toggle("is-hidden", mode !== "ollama");
    if ($("wrap-cloud-base-url")) $("wrap-cloud-base-url").classList.toggle("is-hidden", prov !== "openai_compatible");
  }

  function updateAiBadgeUI(info) {
    const aiPill = $("btn-ai-status");
    const aiText = $("ai-status-text");
    if (!aiPill || !aiText) return;

    if (!info || typeof info !== "object") {
      aiPill.classList.add("is-ai-ready");
      aiPill.classList.remove("is-ai-warn");
      aiText.textContent = "Match Engine: Built-in Smart";
      return;
    }

    const mode = info.matcher_mode || "builtin";
    const isReady =
      mode === "builtin" ||
      (mode === "cloud" && !!info.cloud_has_key) ||
      (mode === "ollama" && !!info.running);

    aiPill.classList.toggle("is-ai-ready", isReady);
    aiPill.classList.toggle("is-ai-warn", !isReady);
    aiText.textContent = info.active_engine_label || "Match Engine: Built-in Smart";
  }

  function applyAiProgressPayload(info) {
    if (!info) return;
    updateAiBadgeUI(info);
    if (typeof info.cache_count === "number" && $("cache-count")) {
      $("cache-count").textContent = String(info.cache_count);
    }

    // Populate matcher dialog only when fields are present
    if (info.matcher_mode) {
      const r = document.querySelector(`input[name="matcher-mode"][value="${info.matcher_mode}"]`);
      if (r) r.checked = true;
    }
    if (info.cloud_provider && $("select-cloud-provider")) {
      $("select-cloud-provider").value = info.cloud_provider;
    }
    if (info.cloud_model && $("input-cloud-model")) {
      $("input-cloud-model").value = info.cloud_model;
    }
    if (typeof info.cloud_base_url === "string" && $("input-cloud-base-url")) {
      $("input-cloud-base-url").value = info.cloud_base_url;
    }
    if (typeof info.cloud_has_key === "boolean") {
      state.clearCloudKeyPending = false;
      if ($("cloud-key-status")) {
        $("cloud-key-status").textContent = info.cloud_has_key
          ? `(Tersimpan: ${info.cloud_key_masked || "••••••••"})`
          : "(Belum ada API Key tersimpan)";
      }
      if ($("btn-clear-cloud-key")) {
        $("btn-clear-cloud-key").classList.toggle("is-hidden", !info.cloud_has_key);
      }
      if ($("input-cloud-api-key")) {
        $("input-cloud-api-key").placeholder = info.cloud_has_key
          ? "Kosongkan jika tetap memakai key tersimpan, atau tempel key baru..."
          : "Tempel API Key Groq / Gemini Anda di sini...";
      }
    }
    syncMatcherSubpanels(info.matcher_mode, info.cloud_provider);

    if (info.model || info.recommended_model) {
      $("ai-dlg-model").textContent = info.model || info.recommended_model || "-";
    }
    if ($("ai-dlg-running")) {
      $("ai-dlg-running").textContent = info.running ? "Aktif (localhost:11434)" : "Belum menyala";
    }
    if ($("ai-dlg-installed")) {
      $("ai-dlg-installed").textContent = info.model_installed ? "Sudah terpasang" : "Belum diunduh";
    }

    const wrap = $("ai-dlg-prog-wrap");
    if (!wrap) return;
    if (info.task_running || info.task_message) {
      wrap.classList.remove("is-hidden");
      $("ai-dlg-prog-status").textContent = info.task_message || "Memproses...";
      const pct = Math.round(info.task_percent || 0);
      $("ai-dlg-prog-pct").textContent = `${pct}%`;
      $("ai-dlg-prog-fill").style.width = `${pct}%`;
    } else {
      wrap.classList.add("is-hidden");
    }
  }

  function handleProgressEvent(ev) {
    if (!ev) return;
    const phase = ev.phase || "fase1_portal";
    const done = Number(ev.current) || 0;
    const total = Number(ev.total) || 0;

    if (phase === "fase3_ai") {
      updateProgressBar("scrape", 1, 1, false);
      updateProgressBar("ai", done, total, total === 0);
    } else if (phase === "fase4_export") {
      updateProgressBar("scrape", 1, 1, false);
      updateProgressBar("ai", 1, 1, false);
    } else {
      updateProgressBar("scrape", done, total, total === 0);
    }

    if ($("kpi-total")) $("kpi-total").textContent = String(ev.kpi_total ?? state.results.length);
    if ($("kpi-free")) $("kpi-free").textContent = String(ev.kpi_free ?? 0);
    if ($("kpi-paid")) $("kpi-paid").textContent = String(ev.kpi_paid ?? 0);
    if ($("kpi-fast")) $("kpi-fast").textContent = String(ev.kpi_fast ?? 0);

    state.latestEtaLabel =
      typeof ev.eta_seconds === "number" && ev.eta_seconds > 0
        ? `ETA ~${formatElapsed(ev.eta_seconds)}`
        : "";
    if ($("elapsed-timer") && typeof ev.elapsed_seconds === "number") {
      $("elapsed-timer").textContent = formatTimerText(ev.elapsed_seconds, state.latestEtaLabel);
    }
    if ($("scrape-rate") && typeof ev.rate_per_sec === "number") {
      $("scrape-rate").textContent = `~${ev.rate_per_sec.toFixed(1)}/dtk`;
    }
    const ticker = $("current-item-ticker");
    if (ticker && ev.current_item) {
      ticker.textContent = ev.current_item;
      ticker.title = ev.current_item;
    }
    updateRunningUI(true, ev.status_text, phase);
  }

  async function bindTauriEvents() {
    if (state.lifecycleBound) return;
    const evApi = getTauriEvent();
    if (!evApi || typeof evApi.listen !== "function") return;
    state.lifecycleBound = true;

    await evApi.listen("scrape-progress", (e) => {
      handleProgressEvent(e.payload);
    });

    await evApi.listen("result-batch", (e) => {
      const batch = Array.isArray(e.payload) ? e.payload : [];
      if (!batch.length) return;
      state.results.push(...batch);
      state.pendingTableRefresh = true;
      if (Date.now() - state.lastTableRenderMs >= 1200) {
        renderResultsTable();
      }
    });

    await evApi.listen("autosave-done", (e) => {
      const p = e.payload || {};
      if (p.path) {
        state.excelPath = p.path;
        if ($("btn-open-excel")) $("btn-open-excel").disabled = false;
        appendLog(`💾 Auto-save checkpoint (${p.count || 0} jurnal) → ${p.path}`);
      }
    });

    await evApi.listen("log-line", (e) => {
      if (typeof e.payload === "string" && e.payload.trim()) {
        appendLog(e.payload);
      }
    });

    await evApi.listen("ai-status-changed", (e) => {
      applyAiProgressPayload(e.payload);
    });

    await evApi.listen("scrape-complete", (e) => {
      const p = e.payload || {};
      if (Array.isArray(p.results)) {
        state.results = p.results;
      }
      if (p.excel_path !== undefined) {
        state.excelPath = p.excel_path || "";
        if ($("btn-open-excel")) $("btn-open-excel").disabled = !state.excelPath;
      }

      if (state.clockTimer) {
        clearInterval(state.clockTimer);
        state.clockTimer = null;
      }

      state.currentPage = 1;
      renderResultsTable();
      updateRunningUI(false, p.cancelled ? "Dihentikan" : "Pencarian selesai", "done");

      const ticker = $("current-item-ticker");
      if (ticker) {
        ticker.textContent = p.cancelled
          ? `Dihentikan (${state.results.length} jurnal parsial).`
          : `Selesai: ${state.results.length} jurnal siap ditinjau di tabel.`;
      }

      const durSec =
        typeof p.duration_sec === "number"
          ? p.duration_sec
          : Math.max(0, (Date.now() - (state.runStartedClientMs || Date.now())) / 1000);
      if (!p.cancelled && state.results.length > 0) {
        showCompletionBanner(durSec, state.results.length);
        playCompletionChime();
      } else if (p.cancelled) {
        showToast("Pencarian dihentikan.");
      }

      refreshAiSetupDialog().catch(() => {});
    });
  }

  async function pollStatusOnce() {
    if (!hasApi()) return;
    try {
      const st = await callBackendApi("aiStatus");
      applyAiProgressPayload(st);
    } catch (e) {}
  }

  async function startSearch(ev) {
    if (ev) ev.preventDefault();
    if (!hasApi()) {
      showToast("Menunggu sambungan Tauri desktop selesai dimuat...");
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
    state.results = [];
    state.excelPath = "";
    if ($("btn-open-excel")) $("btn-open-excel").disabled = true;
    state.currentPage = 1;
    state.pendingTableRefresh = false;
    state.lastTableRenderMs = 0;
    state.runStartedClientMs = Date.now();
    state.latestEtaLabel = "";
    renderResultsTable([]);
    if ($("completion-banner")) $("completion-banner").classList.add("is-hidden");
    if ($("elapsed-timer")) $("elapsed-timer").textContent = "00:00";
    if ($("scrape-rate")) $("scrape-rate").textContent = "~0.0/dtk";
    if ($("current-item-ticker")) $("current-item-ticker").textContent = `Menghubungi portal untuk '${params.keyword}'...`;
    updateRunningUI(true, "Menjelajahi halaman portal SINTA & Garuda...", "fase1_portal");
    updateProgressBar("scrape", 0, 0, true);
    updateProgressBar("ai", 0, 0, false);

    try {
      const res = await callBackendApi("search", params);
      if (res && res.ok === false) {
        updateRunningUI(false);
        showToast(res.message || "Gagal memulai pencarian.");
        return;
      }
      if (!state.clockTimer) {
        state.clockTimer = setInterval(() => {
          if (state.running && $("elapsed-timer") && state.runStartedClientMs > 0) {
            $("elapsed-timer").textContent = formatTimerText((Date.now() - state.runStartedClientMs) / 1000, state.latestEtaLabel);
          }
        }, 500);
      }
    } catch (err) {
      updateRunningUI(false);
      showToast(String(err || "Gagal memulai pencarian."));
    }
  }

  function applyManuscriptResult(res) {
    if (res && res.path) {
      state.manuscriptPath = res.path;
      $("manuscript-name").textContent = res.name || res.path;
      $("manuscript-banner").classList.remove("is-hidden");
      $("btn-clear-manuscript").classList.remove("is-hidden");
      if (res.suggested_keyword && !$("input-keyword").value.trim()) {
        $("input-keyword").value = res.suggested_keyword;
      }
      appendLog(`Naskah dilampirkan: ${res.name || res.path}`);
      showToast(`Naskah '${res.name || "artikel"}' siap dianalisis!`);
    } else if (res && res.message) {
      showToast(res.message);
    }
  }

  function arrayBufferToBase64(buffer) {
    const bytes = new Uint8Array(buffer);
    let binary = "";
    const chunkSize = 0x8000;
    for (let i = 0; i < bytes.length; i += chunkSize) {
      binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
    }
    return btoa(binary);
  }

  async function uploadManuscript() {
    if (!hasApi()) return;
    if ($("input-manuscript-file")) {
      $("input-manuscript-file").click();
    }
  }

  async function refreshAiSetupDialog() {
    if (!hasApi()) return;
    const info = await callBackendApi("aiStatus");
    applyAiProgressPayload(info);
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
        await callBackendApi("stop");
        showToast("Menghentikan pencarian...");
      }
    });

    $("btn-pick-manuscript").addEventListener("click", uploadManuscript);
    if ($("input-manuscript-file")) {
      $("input-manuscript-file").addEventListener("change", async (ev) => {
        const inputEl = ev.target;
        const file = inputEl.files && inputEl.files[0];
        if (!file) return;
        try {
          const buf = await file.arrayBuffer();
          const b64 = arrayBufferToBase64(buf);
          const res = await callBackendApi("uploadManuscript", {
            filename: file.name,
            bytesB64: b64,
          });
          applyManuscriptResult(res);
        } catch (err) {
          showToast("Gagal mengunggah naskah. Pastikan format .docx atau .pdf.");
        } finally {
          inputEl.value = "";
        }
      });
    }
    $("btn-clear-manuscript").addEventListener("click", () => {
      state.manuscriptPath = "";
      $("manuscript-banner").classList.add("is-hidden");
      $("btn-clear-manuscript").classList.add("is-hidden");
      if (hasApi()) {
        callBackendApi("uploadManuscript", { clear: true }).catch(() => {});
      }
    });

    $("btn-toggle-log").addEventListener("click", () => {
      const drawer = $("technical-log-drawer");
      const hidden = drawer.classList.toggle("is-hidden");
      $("btn-toggle-log").setAttribute("aria-expanded", hidden ? "false" : "true");
    });

    $("btn-clear-cache").addEventListener("click", async () => {
      if (!hasApi()) return;
      const res = await callBackendApi("clearCache");
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
      const res = await callBackendApi("copyShare", visibleIndices);
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
      const res = await callBackendApi("copyDraft", {
        index: idx,
        keyword: $("input-keyword").value.trim(),
      });
      showToast(res && res.ok ? "Draf pesan editor disalin ke clipboard!" : "Gagal menyalin draf pesan.");
    };

    $("btn-copy-editor").addEventListener("click", () => copyEditorHandler(state.selectedIndex));
    $("btn-detail-chat").addEventListener("click", () => copyEditorHandler(state.detailIndex));

    $("btn-detail-open-url").addEventListener("click", () => {
      const item = state.results[state.detailIndex];
      if (hasApi() && item && item.url) callBackendApi("openUrl", item.url);
    });

    $("btn-open-excel").addEventListener("click", async () => {
      if (hasApi()) {
        const res = await callBackendApi("openExcel");
        if (res && res.ok === false) showToast(res.message || "File Excel belum tersedia.");
      }
    });

    $("btn-open-folder").addEventListener("click", async () => {
      if (hasApi()) await callBackendApi("openOutput");
    });

    const btnOpenLogs = $("btn-open-logs");
    if (btnOpenLogs) {
      btnOpenLogs.addEventListener("click", async () => {
        if (hasApi()) {
          await callBackendApi("openLogs");
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
      const res = await callBackendApi("launchOllama");
      showToast(res && res.message ? res.message : "Memeriksa layanan Ollama...");
      refreshAiSetupDialog();
    });
    $("btn-ai-install").addEventListener("click", async () => {
      if (!hasApi()) return;
      const res = await callBackendApi("installOllama");
      if (res && res.message) showToast(res.message);
    });
    $("btn-ai-pull").addEventListener("click", async () => {
      if (!hasApi()) return;
      const res = await callBackendApi("pullAiModel");
      if (res && res.message) showToast(res.message);
    });

    document.querySelectorAll('input[name="matcher-mode"]').forEach((radio) => {
      radio.addEventListener("change", () => syncMatcherSubpanels());
    });

    if ($("select-cloud-provider")) {
      $("select-cloud-provider").addEventListener("change", (e) => {
        const prov = e.target.value;
        if ($("input-cloud-model")) {
          $("input-cloud-model").value = defaultCloudModelFor(prov);
          $("input-cloud-model").placeholder = defaultCloudModelFor(prov);
        }
        syncMatcherSubpanels(null, prov);
      });
    }

    if ($("btn-clear-cloud-key")) {
      $("btn-clear-cloud-key").addEventListener("click", () => {
        state.clearCloudKeyPending = true;
        if ($("input-cloud-api-key")) $("input-cloud-api-key").value = "";
        if ($("cloud-key-status")) $("cloud-key-status").textContent = "(Akan dihapus saat disimpan)";
        $("btn-clear-cloud-key").classList.add("is-hidden");
      });
    }

    const collectMatcherForm = () => ({
      mode: document.querySelector('input[name="matcher-mode"]:checked')?.value || "builtin",
      cloud_provider: $("select-cloud-provider")?.value || "groq",
      cloud_model: $("input-cloud-model")?.value.trim() || "",
      cloud_base_url: $("input-cloud-base-url")?.value.trim() || "",
      cloud_api_key: $("input-cloud-api-key")?.value.trim() || "",
      clear_api_key: state.clearCloudKeyPending,
    });

    if ($("btn-test-cloud")) {
      $("btn-test-cloud").addEventListener("click", async () => {
        if (!hasApi()) return;
        const msgBox = $("cloud-test-result");
        if (msgBox) {
          msgBox.className = "cloud-test-msg muted mt-10";
          msgBox.textContent = "Menguji koneksi Cloud AI...";
          msgBox.classList.remove("is-hidden");
        }
        try {
          const res = await callBackendApi("testCloudApi", collectMatcherForm());
          if (msgBox) {
            msgBox.className = res && res.ok ? "cloud-test-msg is-ok mt-10" : "cloud-test-msg is-err mt-10";
            msgBox.textContent = (res && res.message) || "Tidak ada respons.";
          }
        } catch (err) {
          if (msgBox) {
            msgBox.className = "cloud-test-msg is-err mt-10";
            msgBox.textContent = String(err || "Gagal menguji koneksi.");
          }
        }
      });
    }

    if ($("btn-save-matcher")) {
      $("btn-save-matcher").addEventListener("click", async () => {
        if (!hasApi()) return;
        try {
          const updated = await callBackendApi("saveMatcherConfig", collectMatcherForm());
          if ($("input-cloud-api-key")) $("input-cloud-api-key").value = "";
          applyAiProgressPayload(updated);
          const dlg = $("dialog-ai-setup");
          if (dlg && dlg.open) dlg.close();
          showToast(`Pengaturan disimpan (${updated?.active_engine_label || "Built-in Smart"}).`);
        } catch (err) {
          showToast(String(err || "Gagal menyimpan pengaturan Match Engine."));
        }
      });
    }
  }

  async function onBridgeReady() {
    applyPreset("skripsi", true);
    applySoundPreference();
    renderResultsTable([]);
    appendLog("JSFS v3.0.1 siap digunakan (Tauri Native Engine).");
    await bindTauriEvents();
    await pollStatusOnce();
    setTimeout(pollStatusOnce, 600);
  }

  document.addEventListener("DOMContentLoaded", () => {
    bindEvents();
    onBridgeReady();
  });
})();

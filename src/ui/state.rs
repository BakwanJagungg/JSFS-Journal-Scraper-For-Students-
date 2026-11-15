//! Shared UI state, quick student presets, table filtering/sorting, and budget parsing helpers.

use crate::engine::ai_ops::AiStatusExtended;
use crate::engine::manuscript::ManuscriptSummary;
use crate::models::{SearchParams, UiJournalItem};
use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportScope {
    #[default]
    All,
    BookmarkedOnly,
    FreeOnly,
    TableFiltered,
}

impl ExportScope {
    pub const ALL: [Self; 4] = [
        Self::All,
        Self::BookmarkedOnly,
        Self::FreeOnly,
        Self::TableFiltered,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "Semua Hasil",
            Self::BookmarkedOnly => "Hanya ★ Pilihan",
            Self::FreeOnly => "Hanya Gratis (Rp 0)",
            Self::TableFiltered => "Sesuai Filter Aktif di Tabel",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::All => "",
            Self::BookmarkedOnly => "pilihan",
            Self::FreeOnly => "gratis",
            Self::TableFiltered => "filtered",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub bookmarked_ids: HashSet<String>,
    pub only_bookmarked: bool,
    pub export_scope: ExportScope,
}

impl AppState {
    pub fn bookmark_key(item: &UiJournalItem) -> String {
        let primary = item.url.trim();
        if !primary.is_empty() {
            return primary.to_ascii_lowercase();
        }
        let garuda = item.garuda_url.trim();
        if !garuda.is_empty() {
            return garuda.to_ascii_lowercase();
        }
        format!("title:{}", item.nama.trim().to_ascii_lowercase())
    }

    pub fn is_bookmarked(&self, item: &UiJournalItem) -> bool {
        self.bookmarked_ids.contains(&Self::bookmark_key(item))
    }

    pub fn toggle_bookmark(&mut self, item: &UiJournalItem) -> bool {
        let key = Self::bookmark_key(item);
        if !self.bookmarked_ids.remove(&key) {
            self.bookmarked_ids.insert(key);
            true
        } else {
            false
        }
    }

    pub fn bookmarked_count(&self, results: &[UiJournalItem]) -> usize {
        results.iter().filter(|it| self.is_bookmarked(it)).count()
    }
}

pub const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuickPreset {
    SkripsiS1,
    JurnalGratis,
    TerbitTerdekat,
    ReputasiTinggi,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApcFilter {
    All,
    Free,
    Paid,
    Unknown,
}

impl ApcFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "Semua Biaya",
            Self::Free => "Gratis (Rp 0)",
            Self::Paid => "Berbayar",
            Self::Unknown => "Cek Web",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    MatchScore,
    Title,
    Sinta,
    Cost,
    Activity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FilterSettings {
    pub active_preset: QuickPreset,
    pub target_count: usize,
    pub sinta_enabled: [bool; 6],
    pub garuda: bool,
    pub sinta_src: bool,
    pub include_non_sinta_garuda: bool,
    pub months: [bool; 12],
    pub include_unknown_month: bool,
    pub min_rp: i64,
    pub max_rp: i64,
    pub min_rp_text: String,
    pub max_rp_text: String,
    pub only_free: bool,
    pub include_na: bool,
    pub exclude_dormant: bool,
    pub match_enabled: bool,
    pub use_ai: bool,
    pub fmt_xlsx: bool,
    pub fmt_md: bool,
    pub fmt_csv: bool,
}

impl Default for FilterSettings {
    fn default() -> Self {
        Self {
            active_preset: QuickPreset::SkripsiS1,
            target_count: 40,
            sinta_enabled: [false, false, true, true, true, false], // SINTA 3, 4, 5
            garuda: true,
            sinta_src: true,
            include_non_sinta_garuda: false,
            months: [true; 12],
            include_unknown_month: true,
            min_rp: 0,
            max_rp: 1_500_000,
            min_rp_text: "0".to_string(),
            max_rp_text: "1500000".to_string(),
            only_free: false,
            include_na: true,
            exclude_dormant: true,
            match_enabled: true,
            use_ai: true,
            fmt_xlsx: true,
            fmt_md: true,
            fmt_csv: false,
        }
    }
}

impl FilterSettings {
    pub fn clamp_target(&mut self, val: usize) {
        self.target_count = val.clamp(1, crate::config::MAX_TARGET_COUNT);
    }

    pub fn sync_budget_from_text(&mut self) {
        self.min_rp = parse_idr_input(&self.min_rp_text).max(0);
        let parsed_max = parse_idr_input(&self.max_rp_text);
        self.max_rp = if parsed_max > 0 {
            parsed_max.max(self.min_rp)
        } else {
            15_000_000
        };
    }

    pub fn apply_preset(&mut self, preset: QuickPreset) -> &'static str {
        self.active_preset = preset;
        match preset {
            QuickPreset::SkripsiS1 => {
                self.sinta_enabled = [false, false, true, true, true, false];
                self.target_count = 40;
                self.only_free = false;
                self.include_na = true;
                self.min_rp = 0;
                self.max_rp = 1_500_000;
                self.min_rp_text = "0".to_string();
                self.max_rp_text = "1500000".to_string();
                self.months = [true; 12];
                "Preset Aktif: Target Skripsi S1 (SINTA 3–5, Rp 0–1,5 jt + Cek Web, target 40)."
            }
            QuickPreset::JurnalGratis => {
                self.sinta_enabled = [false, true, true, true, true, true];
                self.target_count = 50;
                self.only_free = true;
                self.include_na = true;
                self.months = [true; 12];
                "Preset Aktif: Jurnal Gratis (Rp 0, SINTA 2–6)."
            }
            QuickPreset::TerbitTerdekat => {
                self.sinta_enabled = [false, false, true, true, true, true];
                self.target_count = 25;
                self.only_free = false;
                self.include_na = true;
                self.select_near_months();
                "Preset Aktif: Terbit Terdekat (3 bulan ke depan, SINTA 3–6)."
            }
            QuickPreset::ReputasiTinggi => {
                self.sinta_enabled = [true, true, false, false, false, false];
                self.target_count = 25;
                self.only_free = false;
                self.include_na = true;
                self.min_rp = 0;
                self.max_rp = 15_000_000;
                self.min_rp_text = "0".to_string();
                self.max_rp_text = "15000000".to_string();
                self.months = [true; 12];
                "Preset Aktif: Reputasi Tinggi (SINTA 1 & 2)."
            }
            QuickPreset::Custom => "Filter kustom diterapkan.",
        }
    }

    pub fn select_near_months(&mut self) {
        let cur = chrono::Local::now().month() as usize; // 1..=12
        self.months = [false; 12];
        for offset in 0..3 {
            let m_idx = (cur - 1 + offset) % 12;
            self.months[m_idx] = true;
        }
    }

    pub fn active_filter_count(&self) -> usize {
        let mut count = 0;
        let sinta_count = self.sinta_enabled.iter().filter(|&&b| b).count();
        let month_count = self.months.iter().filter(|&&b| b).count();
        if sinta_count > 0 && sinta_count < 6 {
            count += 1;
        }
        if month_count > 0 && month_count < 12 {
            count += 1;
        }
        if self.only_free {
            count += 1;
        }
        if self.max_rp > 0 && self.max_rp < 15_000_000 {
            count += 1;
        }
        if !self.garuda || !self.sinta_src {
            count += 1;
        }
        if self.include_non_sinta_garuda {
            count += 1;
        }
        if !self.exclude_dormant {
            count += 1;
        }
        count
    }

    pub fn summary_text(&self) -> String {
        let ranks: Vec<usize> = self
            .sinta_enabled
            .iter()
            .enumerate()
            .filter_map(|(i, &on)| on.then_some(i + 1))
            .collect();
        let sinta_str = if ranks.is_empty() {
            "Non-SINTA".to_string()
        } else if ranks.len() == 6 {
            "SINTA 1-6".to_string()
        } else {
            let first = ranks[0];
            let last = ranks[ranks.len() - 1];
            if last - first + 1 == ranks.len() && ranks.len() > 1 {
                format!("SINTA {first}-{last}")
            } else {
                let joined: Vec<String> = ranks.iter().map(ToString::to_string).collect();
                format!("SINTA {}", joined.join(","))
            }
        };

        let cost_str = if self.only_free {
            "Gratis Rp 0".to_string()
        } else if self.max_rp > 0 && self.max_rp < 15_000_000 {
            if self.min_rp <= 0 {
                format!("<= Rp {}", format_compact_idr(self.max_rp))
            } else {
                format!(
                    "Rp {}-{}",
                    format_compact_idr(self.min_rp),
                    format_compact_idr(self.max_rp)
                )
            }
        } else {
            "Semua Biaya".to_string()
        };

        let months: Vec<&str> = self
            .months
            .iter()
            .enumerate()
            .filter_map(|(i, &on)| on.then_some(MONTH_NAMES[i]))
            .collect();
        let month_str = match months.len() {
            12 => "Semua Bulan".to_string(),
            0 => "Tanpa Bulan".to_string(),
            1..=4 => months.join(", "),
            n => format!("{n} Bulan"),
        };

        format!("Filter: {sinta_str} • {cost_str} • {month_str}")
    }

    pub fn to_search_params(&mut self, keyword: &str, ollama_model: &str) -> SearchParams {
        self.sync_budget_from_text();
        let months: Vec<serde_json::Value> = self
            .months
            .iter()
            .enumerate()
            .filter_map(|(i, &on)| on.then_some(serde_json::json!(i + 1)))
            .collect();
        let sinta_tiers: Vec<serde_json::Value> = self
            .sinta_enabled
            .iter()
            .enumerate()
            .filter_map(|(i, &on)| on.then_some(serde_json::json!(i + 1)))
            .collect();
        let mut formats = Vec::new();
        if self.fmt_xlsx {
            formats.push(".xlsx".to_string());
        }
        if self.fmt_md {
            formats.push(".md".to_string());
        }
        if self.fmt_csv {
            formats.push(".csv".to_string());
        }
        if formats.is_empty() {
            formats.push(".xlsx".to_string());
        }

        let clean_kw = keyword.trim().to_string();
        SearchParams {
            keywords: clean_kw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect(),
            keyword: clean_kw,
            target_count: self.target_count.clamp(1, crate::config::MAX_TARGET_COUNT),
            months,
            sinta_tiers,
            use_ai: self.use_ai,
            ai_model: (!ollama_model.trim().is_empty()).then_some(ollama_model.to_string()),
            garuda: self.garuda,
            sinta_src: self.sinta_src,
            only_free: self.only_free,
            include_na: self.include_na,
            include_unknown: self.include_unknown_month,
            min_rp: self.min_rp,
            max_rp: self.max_rp,
            match_enabled: self.match_enabled,
            include_non_sinta_garuda: self.include_non_sinta_garuda,
            exclude_dormant: self.exclude_dormant,
            formats,
        }
    }
}

pub fn parse_idr_input(raw: &str) -> i64 {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<i64>().unwrap_or(0)
}

fn format_compact_idr(amount: i64) -> String {
    if amount <= 0 {
        return "0".to_string();
    }
    if amount >= 1_000_000 {
        let jt = amount / 1_000_000;
        let rem = (amount % 1_000_000) / 100_000;
        if rem > 0 {
            format!("{jt},{rem} jt")
        } else {
            format!("{jt} jt")
        }
    } else if amount >= 1_000 {
        format!("{} rb", amount / 1_000)
    } else {
        amount.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct MatcherFormState {
    pub mode: String,
    pub cloud_provider: String,
    pub cloud_model: String,
    pub cloud_base_url: String,
    pub cloud_api_key_input: String,
    pub clear_api_key: bool,
    pub ollama_model: String,
    pub cloud_test_result: Option<(bool, String)>,
    pub pull_running: bool,
    pub pull_percent: f64,
    pub pull_message: String,
}

impl Default for MatcherFormState {
    fn default() -> Self {
        Self {
            mode: "builtin".to_string(),
            cloud_provider: "groq".to_string(),
            cloud_model: crate::config::DEFAULT_GROQ_MODEL.to_string(),
            cloud_base_url: "https://api.openai.com/v1".to_string(),
            cloud_api_key_input: String::new(),
            clear_api_key: false,
            ollama_model: crate::config::DEFAULT_AI_MODEL.to_string(),
            cloud_test_result: None,
            pull_running: false,
            pull_percent: 0.0,
            pull_message: String::new(),
        }
    }
}

impl MatcherFormState {
    pub fn sync_from_status(&mut self, st: &AiStatusExtended) {
        self.mode = st.matcher_mode.clone();
        self.cloud_provider = st.cloud_provider.clone();
        self.cloud_model = st.cloud_model.clone();
        self.cloud_base_url = st.cloud_base_url.clone();
        self.ollama_model = st.ollama_model.clone();
        self.clear_api_key = false;
    }
}

#[derive(Debug, Clone)]
pub struct SearchProgressView {
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub percent: f64,
    pub current_item: String,
    pub status_text: String,
    pub rate_per_sec: f64,
    pub elapsed_seconds: f64,
    pub eta_seconds: Option<u64>,
    pub kpi_total: usize,
    pub kpi_free: usize,
    pub kpi_paid: usize,
    pub kpi_fast: usize,
}

impl Default for SearchProgressView {
    fn default() -> Self {
        Self {
            phase: String::new(),
            current: 0,
            total: 0,
            percent: 0.0,
            current_item: "Masukkan topik lalu klik Mulai Cari Jurnal".to_string(),
            status_text: "Siap menjelajahi SINTA & Garuda".to_string(),
            rate_per_sec: 0.0,
            elapsed_seconds: 0.0,
            eta_seconds: None,
            kpi_total: 0,
            kpi_free: 0,
            kpi_paid: 0,
            kpi_fast: 0,
        }
    }
}

pub fn format_duration(secs: f64) -> String {
    let s = secs.max(0.0).floor() as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

pub fn activity_badge_label(item: &UiJournalItem) -> String {
    if item.is_dormant {
        item.last_issue_year
            .map(|y| format!("Dorman {y}"))
            .unwrap_or_else(|| "Dorman".to_string())
    } else if item.activity_status.contains("Aktif") {
        item.last_issue_year
            .map(|y| format!("Aktif {y}"))
            .unwrap_or_else(|| "Aktif".to_string())
    } else if let Some(y) = item.last_issue_year {
        format!("Terakhir {y}")
    } else {
        item.activity_status.clone()
    }
}

/// Filters & sorts `results` indices without duplicating underlying `UiJournalItem` allocations.
pub fn filtered_sorted_indices(
    results: &[UiJournalItem],
    query: &str,
    apc_filter: ApcFilter,
    bookmarked_only: Option<&HashSet<String>>,
    sort_col: SortColumn,
    sort_desc: bool,
) -> Vec<usize> {
    let q = query.trim().to_ascii_lowercase();
    let mut indices: Vec<usize> = results
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            if let Some(bm) = bookmarked_only {
                if !bm.contains(&AppState::bookmark_key(r)) {
                    return false;
                }
            }
            let apc_ok = match apc_filter {
                ApcFilter::All => true,
                ApcFilter::Free => r.apc_category == "free",
                ApcFilter::Paid => r.apc_category == "paid",
                ApcFilter::Unknown => r.apc_category == "unknown",
            };
            if !apc_ok {
                return false;
            }
            if q.is_empty() {
                return true;
            }
            r.nama.to_ascii_lowercase().contains(&q)
                || r.penerbit.to_ascii_lowercase().contains(&q)
                || r.email.to_ascii_lowercase().contains(&q)
                || r.focus_scope.to_ascii_lowercase().contains(&q)
        })
        .map(|(i, _)| i)
        .collect();

    indices.sort_by(|&a, &b| {
        let ra = &results[a];
        let rb = &results[b];
        let ord = match sort_col {
            SortColumn::MatchScore => ra
                .match_score
                .total_cmp(&rb.match_score)
                .then_with(|| rb.biaya_sort.cmp(&ra.biaya_sort)),
            SortColumn::Title => ra
                .nama
                .to_ascii_lowercase()
                .cmp(&rb.nama.to_ascii_lowercase()),
            SortColumn::Sinta => {
                let sa = ra.sinta.parse::<u8>().unwrap_or(99);
                let sb = rb.sinta.parse::<u8>().unwrap_or(99);
                sa.cmp(&sb)
            }
            SortColumn::Cost => ra.biaya_sort.cmp(&rb.biaya_sort),
            SortColumn::Activity => {
                let ya = ra.last_issue_year.unwrap_or(0);
                let yb = rb.last_issue_year.unwrap_or(0);
                ya.cmp(&yb)
            }
        };
        if sort_desc {
            ord.reverse()
        } else {
            ord
        }
    });

    indices
}

pub fn manuscript_status_text(ms: Option<&ManuscriptSummary>) -> String {
    if let Some(m) = ms {
        format!("Naskah: {} ({} kar.)", m.name, m.char_count)
    } else {
        "Belum ada naskah terlampir".to_string()
    }
}

pub const MAX_SEARCH_HISTORY: usize = 15;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchHistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub keyword: String,
    pub filter_summary: String,
    pub filter_snapshot: FilterSettings,
    pub total_found: usize,
    pub free_count: usize,
    pub records: Vec<UiJournalItem>,
}

pub fn load_search_history_from(path: &Path) -> Vec<SearchHistoryEntry> {
    if let Ok(raw) = std::fs::read_to_string(path) {
        if let Ok(mut items) = serde_json::from_str::<Vec<SearchHistoryEntry>>(&raw) {
            items.truncate(MAX_SEARCH_HISTORY);
            return items;
        }
    }
    Vec::new()
}

pub fn save_search_history_to(path: &Path, entries: &[SearchHistoryEntry]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let slice = if entries.len() > MAX_SEARCH_HISTORY {
        &entries[..MAX_SEARCH_HISTORY]
    } else {
        entries
    };
    let json = serde_json::to_string_pretty(slice)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

pub fn load_search_history() -> Vec<SearchHistoryEntry> {
    load_search_history_from(&crate::config::search_history_path())
}

pub fn push_search_history_entry(
    history: &mut Vec<SearchHistoryEntry>,
    keyword: &str,
    filters: &FilterSettings,
    results: &[UiJournalItem],
) {
    let now = Local::now();
    let entry = SearchHistoryEntry {
        id: now.format("%Y%m%d%H%M%S%3f").to_string(),
        timestamp: now.format("%d %b %Y %H:%M").to_string(),
        keyword: keyword.trim().to_string(),
        filter_summary: filters.summary_text(),
        filter_snapshot: filters.clone(),
        total_found: results.len(),
        free_count: results.iter().filter(|r| r.apc_category == "free").count(),
        records: results.to_vec(),
    };
    history.insert(0, entry);
    history.truncate(MAX_SEARCH_HISTORY);
}

pub fn record_search_history(
    history: &mut Vec<SearchHistoryEntry>,
    keyword: &str,
    filters: &FilterSettings,
    results: &[UiJournalItem],
) {
    push_search_history_entry(history, keyword, filters, results);
    let _ = save_search_history_to(&crate::config::search_history_path(), history);
}

pub fn select_records_for_scope<'a>(
    results: &'a [UiJournalItem],
    scope: ExportScope,
    app_state: &AppState,
    table_filter: &str,
    apc_filter: ApcFilter,
    sort_col: SortColumn,
    sort_desc: bool,
) -> Vec<&'a UiJournalItem> {
    match scope {
        ExportScope::All => results.iter().collect(),
        ExportScope::BookmarkedOnly => results
            .iter()
            .filter(|r| app_state.is_bookmarked(r))
            .collect(),
        ExportScope::FreeOnly => results
            .iter()
            .filter(|r| r.apc_category == "free")
            .collect(),
        ExportScope::TableFiltered => {
            let bm_filter = app_state
                .only_bookmarked
                .then_some(&app_state.bookmarked_ids);
            let indices = filtered_sorted_indices(
                results,
                table_filter,
                apc_filter,
                bm_filter,
                sort_col,
                sort_desc,
            );
            indices.into_iter().filter_map(|i| results.get(i)).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_skripsi_and_gratis() {
        let mut fs = FilterSettings::default();
        assert!(!fs.only_free);
        assert!(fs.include_na);
        assert_eq!(fs.min_rp, 0);
        assert_eq!(fs.max_rp, 1_500_000);
        assert!((25..=60).contains(&fs.target_count));

        fs.apply_preset(QuickPreset::SkripsiS1);
        assert!(!fs.only_free);
        assert!(fs.include_na);
        assert_eq!(fs.max_rp, 1_500_000);

        fs.apply_preset(QuickPreset::JurnalGratis);
        assert!(fs.only_free);
        assert_eq!(fs.target_count, 50);
        assert!(!fs.sinta_enabled[0]);
        assert!(fs.sinta_enabled[1]);

        fs.apply_preset(QuickPreset::ReputasiTinggi);
        assert!(!fs.only_free);
        assert!(fs.sinta_enabled[0]);
        assert!(fs.sinta_enabled[1]);
        assert!(!fs.sinta_enabled[2]);
    }

    #[test]
    fn test_parse_idr_input_formatted_strings() {
        assert_eq!(parse_idr_input("1.500.000"), 1_500_000);
        assert_eq!(parse_idr_input("Rp 350.000"), 350_000);
        assert_eq!(parse_idr_input("0"), 0);
    }

    #[test]
    fn test_filtered_sorted_indices_and_activity_badge() {
        let rows = vec![
            UiJournalItem {
                nama: "Jurnal Beta".to_string(),
                apc_category: "paid".to_string(),
                biaya_sort: 500_000,
                match_score: 70.0,
                last_issue_year: Some(2024),
                activity_status: "Terakhir Terbit 2024".to_string(),
                ..Default::default()
            },
            UiJournalItem {
                nama: "Jurnal Alpha".to_string(),
                apc_category: "free".to_string(),
                biaya_sort: 0,
                match_score: 92.0,
                last_issue_year: Some(2026),
                activity_status: "Aktif (2026)".to_string(),
                ..Default::default()
            },
        ];

        assert_eq!(activity_badge_label(&rows[1]), "Aktif 2026");
        assert_eq!(activity_badge_label(&rows[0]), "Terakhir 2024");

        let only_free = filtered_sorted_indices(
            &rows,
            "",
            ApcFilter::Free,
            None,
            SortColumn::MatchScore,
            true,
        );
        assert_eq!(only_free, vec![1]);

        let by_cost =
            filtered_sorted_indices(&rows, "", ApcFilter::All, None, SortColumn::Cost, false);
        assert_eq!(by_cost, vec![1, 0]);
    }

    #[test]
    fn test_bookmark_shortlist_and_filter_summary() {
        let mut fs = FilterSettings::default();
        fs.apply_preset(QuickPreset::JurnalGratis);
        assert_eq!(
            fs.summary_text(),
            "Filter: SINTA 2-6 • Gratis Rp 0 • Semua Bulan"
        );

        fs.apply_preset(QuickPreset::SkripsiS1);
        assert_eq!(
            fs.summary_text(),
            "Filter: SINTA 3-5 • <= Rp 1,5 jt • Semua Bulan"
        );

        let rows = vec![
            UiJournalItem {
                nama: "Jurnal A".to_string(),
                url: "https://a.example.ac.id".to_string(),
                match_score: 80.0,
                ..Default::default()
            },
            UiJournalItem {
                nama: "Jurnal B".to_string(),
                url: "https://b.example.ac.id".to_string(),
                match_score: 70.0,
                ..Default::default()
            },
        ];
        let mut app_state = AppState::default();
        assert_eq!(app_state.bookmarked_count(&rows), 0);
        assert!(app_state.toggle_bookmark(&rows[1]));
        assert!(!app_state.is_bookmarked(&rows[0]));
        assert!(app_state.is_bookmarked(&rows[1]));
        assert_eq!(app_state.bookmarked_count(&rows), 1);

        let shortlisted = filtered_sorted_indices(
            &rows,
            "",
            ApcFilter::All,
            Some(&app_state.bookmarked_ids),
            SortColumn::MatchScore,
            true,
        );
        assert_eq!(shortlisted, vec![1]);
    }

    #[test]
    fn test_search_history_persistence_and_targeted_scope() {
        let mut fs = FilterSettings::default();
        fs.apply_preset(QuickPreset::JurnalGratis);

        let rows = vec![
            UiJournalItem {
                nama: "Jurnal Gratis Favorit".to_string(),
                url: "https://gratis.example.ac.id".to_string(),
                apc_category: "free".to_string(),
                email: "editor@gratis.ac.id, invalid-email".to_string(),
                ..Default::default()
            },
            UiJournalItem {
                nama: "Jurnal Berbayar".to_string(),
                url: "https://berbayar.example.ac.id".to_string(),
                apc_category: "paid".to_string(),
                email: "editor@gratis.ac.id; redaksi@berbayar.ac.id".to_string(),
                ..Default::default()
            },
        ];

        let mut app_state = AppState::default();
        app_state.toggle_bookmark(&rows[0]);

        let all = select_records_for_scope(
            &rows,
            ExportScope::All,
            &app_state,
            "",
            ApcFilter::All,
            SortColumn::MatchScore,
            true,
        );
        assert_eq!(all.len(), 2);

        let bm = select_records_for_scope(
            &rows,
            ExportScope::BookmarkedOnly,
            &app_state,
            "",
            ApcFilter::All,
            SortColumn::MatchScore,
            true,
        );
        assert_eq!(bm.len(), 1);
        assert_eq!(bm[0].nama, "Jurnal Gratis Favorit");

        let free = select_records_for_scope(
            &rows,
            ExportScope::FreeOnly,
            &app_state,
            "",
            ApcFilter::All,
            SortColumn::MatchScore,
            true,
        );
        assert_eq!(free.len(), 1);

        let (emails_str, email_count) = crate::engine::file_ops::collect_valid_emails(&all);
        assert_eq!(email_count, 2);
        assert_eq!(emails_str, "editor@gratis.ac.id, redaksi@berbayar.ac.id");

        let tmp = std::env::temp_dir().join("jsfs_test_history_v420.json");
        let _ = std::fs::remove_file(&tmp);
        let mut hist = Vec::new();
        for i in 0..20 {
            push_search_history_entry(&mut hist, &format!("topik {i}"), &fs, &rows);
        }
        assert_eq!(hist.len(), MAX_SEARCH_HISTORY);
        assert_eq!(hist[0].keyword, "topik 19");
        save_search_history_to(&tmp, &hist).unwrap();

        let loaded = load_search_history_from(&tmp);
        assert_eq!(loaded.len(), MAX_SEARCH_HISTORY);
        assert_eq!(loaded[0].keyword, "topik 19");
        assert_eq!(loaded[0].free_count, 1);
        assert_eq!(loaded[0].records.len(), 2);
        let _ = std::fs::remove_file(&tmp);
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DisplayCurrency {
    #[default]
    Idr,
    Usd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitStatus {
    Sesuai,
    TerlaluPendek,
    MelebihiBatas,
}

impl DisplayCurrency {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::Idr => "Rupiah (IDR)",
            Self::Usd => "Dollar (USD)",
        }
    }
}

/// Canonical journal metadata record shared between backend, SQLite cache, exporters, and UI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JournalRecord {
    pub title: String,
    pub publisher: String,
    pub issn: String,
    pub eissn: String,
    pub garuda_url: String,
    pub website_url: String,
    pub contact_url: String,
    pub about_url: String,
    pub editorial_url: String,
    pub sinta_score: String,
    pub subject_area: String,
    pub focus_and_scope: String,
    pub publication_fee: String,
    pub fee_currency: String,
    pub fee_amount: String,
    pub apc_status: String,
    pub apc_raw_text: String,
    pub email: String,
    pub phone: String,
    pub publish_schedule: String,
    pub publish_months: Vec<String>,
    pub next_issue_estimate: String,
    pub ai_validated: bool,
    pub ai_source: String,
    pub is_incomplete: bool,
    pub match_score: f64,
    pub match_label: String,
    pub match_reasons: Vec<String>,
    pub cache_version: String,

    #[serde(default)]
    pub sinta_url: String,
    #[serde(default)]
    pub source_portal: String,
    #[serde(default)]
    pub publication_fee_idr: Option<i64>,
    #[serde(default)]
    pub apc_idr_numeric: Option<u64>,
    #[serde(default)]
    pub submission_fee_idr: Option<u64>,
    #[serde(default)]
    pub publish_month_nums: Vec<u8>,
    #[serde(default)]
    pub server_status: String,
    #[serde(default)]
    pub last_issue_year: Option<u16>,
    #[serde(default)]
    pub activity_status: String,
    #[serde(default)]
    pub is_dormant: bool,
    #[serde(default)]
    pub bot_protected: bool,
    #[serde(default)]
    pub issn_valid: bool,
    #[serde(default)]
    pub extra_indexes: Vec<String>,
    #[serde(default)]
    pub apc_evidence: String,
    #[serde(default)]
    pub quartile: Option<String>,
    #[serde(default)]
    pub is_international: bool,
    #[serde(default)]
    pub turnaround_weeks: Option<u16>,
    #[serde(default)]
    pub turnaround_label: String,
    #[serde(default)]
    pub article_languages: String,
    #[serde(default)]
    pub template_url: Option<String>,
    #[serde(default)]
    pub risk_warning: Option<String>,
    #[serde(default)]
    pub citation_style: Option<String>,
    #[serde(default)]
    pub word_limit: Option<(u32, u32)>,
    #[serde(default)]
    pub cited_count: u32,
    #[serde(default)]
    pub submission_open: Option<bool>,
    #[serde(default)]
    pub editor_in_chief: Option<String>,
    #[serde(default)]
    pub max_similarity_pct: Option<u8>,
}

/// Compute human-readable journal activity label and dormancy flag against `current_year`.
pub fn compute_activity_status(
    last_issue_year: Option<u16>,
    broken_archive: bool,
    current_year: u16,
) -> (String, bool) {
    match last_issue_year {
        Some(y) if y >= current_year.saturating_sub(1) => (format!("Aktif ({y})"), false),
        Some(y) if y >= current_year.saturating_sub(2) => {
            (format!("Kurang Aktif (Terakhir {y})"), false)
        }
        Some(y) => (format!("Dorman / Tidak Terurus ({y})"), true),
        None if broken_archive => (
            "Dorman / Tidak Terurus (Arsip Kosong/Rusak)".to_string(),
            true,
        ),
        None => ("Belum Terdeteksi".to_string(), false),
    }
}

impl JournalRecord {
    /// Update `activity_status` and `is_dormant` from `self.last_issue_year`.
    pub fn refresh_activity_status(&mut self, broken_archive: bool, current_year: u16) {
        if self.bot_protected && self.last_issue_year.is_none() && !broken_archive {
            self.activity_status = "Proteksi Bot (Buka Web)".to_string();
            self.is_dormant = false;
            return;
        }
        let (status, dormant) =
            compute_activity_status(self.last_issue_year, broken_archive, current_year);
        self.activity_status = status;
        self.is_dormant = dormant;
    }
    /// Classify APC into `"free"`, `"paid"`, or `"unknown"` consistently for UI/filter/sort.
    pub fn apc_category(&self) -> &'static str {
        let s = self.apc_status.to_lowercase();
        if self.apc_idr_numeric == Some(0)
            || self.publication_fee_idr == Some(0)
            || s.contains("gratis")
            || s.starts_with("free")
            || s.contains("rp 0")
            || s.contains("diamond oa")
            || s.contains("$0")
            || s.contains("bebas biaya")
        {
            return "free";
        }
        if self.apc_idr_numeric.is_some_and(|v| v > 0)
            || self.publication_fee_idr.is_some_and(|v| v > 0)
        {
            return "paid";
        }
        let f = self.fee_amount.trim();
        if !f.is_empty() && f.parse::<f64>().is_ok_and(|v| v > 0.0) {
            return "paid";
        }
        "unknown"
    }

    /// Resolve pure numeric APC in IDR (`Some(0)` for free, `Some(n)` for paid, `None` for unknown).
    pub fn resolved_apc_idr_numeric(&self) -> Option<u64> {
        if self.apc_category() == "free" {
            return Some(0);
        }
        self.apc_idr_numeric.or_else(|| {
            self.publication_fee_idr
                .filter(|&v| v >= 0)
                .map(|v| v as u64)
        })
    }

    /// Format and apply the active single-currency APC label without losing `apc_idr_numeric`.
    pub fn apply_display_currency(&mut self, currency: DisplayCurrency, usd_to_idr: i64) {
        let numeric = self.resolved_apc_idr_numeric();
        if self.apc_idr_numeric.is_none() {
            self.apc_idr_numeric = numeric;
        }
        if self.publication_fee_idr.is_none() {
            self.publication_fee_idr = numeric.map(|v| v as i64);
        }
        let formatted = format_apc_unified(
            self.apc_category(),
            numeric,
            &self.apc_status,
            currency,
            usd_to_idr,
        );
        self.apc_status = formatted.clone();
        self.publication_fee = formatted;
    }

    /// Extract SINTA numeric rank 1..=6 if available.
    pub fn sinta_rank_num(&self) -> Option<u8> {
        let up = self.sinta_score.to_uppercase();
        for ch in ['1', '2', '3', '4', '5', '6'] {
            if up.contains(&format!("S{ch}"))
                || up.contains(&format!("SINTA {ch}"))
                || up.trim() == ch.to_string()
            {
                return ch.to_digit(10).map(|d| d as u8);
            }
        }
        None
    }

    /// Extract international quartile numeric rank (`Q1` -> 1 .. `Q4` -> 4).
    pub fn quartile_rank_num(&self) -> Option<u8> {
        match self.quartile.as_deref().map(str::trim) {
            Some(q) if q.eq_ignore_ascii_case("Q1") => Some(1),
            Some(q) if q.eq_ignore_ascii_case("Q2") => Some(2),
            Some(q) if q.eq_ignore_ascii_case("Q3") => Some(3),
            Some(q) if q.eq_ignore_ascii_case("Q4") => Some(4),
            _ => None,
        }
    }

    /// Human-readable accreditation label covering Q1–Q4 international quartiles and SINTA 1–6.
    pub fn accreditation_label(&self) -> String {
        match (
            self.sinta_rank_num(),
            self.quartile
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
        ) {
            (Some(s), Some(q)) => format!("SINTA {s} / {}", q.to_ascii_uppercase()),
            (None, Some(q)) => q.to_ascii_uppercase(),
            (Some(s), None) => format!("SINTA {s}"),
            (None, None) if self.is_international => "Intl OA".to_string(),
            (None, None) => "Garuda".to_string(),
        }
    }

    /// Sort key in IDR for APC column (`0` for free, numeric IDR for paid, `i64::MAX` for unknown).
    pub fn biaya_sort_idr(&self) -> i64 {
        match self.apc_category() {
            "free" => 0,
            "paid" => self.publication_fee_idr.unwrap_or_else(|| {
                self.fee_amount
                    .replace(['.', ','], "")
                    .trim()
                    .parse::<i64>()
                    .unwrap_or(i64::MAX - 1)
            }),
            _ => i64::MAX,
        }
    }
}

/// UI-friendly payload serialized to match JSFS v2.4 frontend table columns while keeping v3 keys.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiJournalItem {
    pub nama: String,
    pub penerbit: String,
    pub sumber: String,
    pub sinta: String,
    pub match_score: f64,
    pub kesesuaian_naskah: String,
    pub match_reasons: Vec<String>,
    pub apc_label: String,
    pub apc_category: String,
    pub submit_label: String,
    pub biaya_publikasi: Option<i64>,
    #[serde(default)]
    pub apc_idr_numeric: Option<u64>,
    pub biaya_sort: i64,
    pub jadwal_terbit: String,
    pub terbit_terdekat: String,
    pub email: String,
    pub email_short: String,
    pub telepon: String,
    pub url: String,
    pub sinta_url: String,
    pub garuda_url: String,
    pub issn: String,
    pub eissn: String,
    pub focus_scope: String,
    pub ai_validated: bool,
    pub ai_source: String,
    pub health_warning: bool,
    pub last_issue_year: Option<u16>,
    pub activity_status: String,
    pub is_dormant: bool,
    pub apc_evidence: String,
    pub quartile: Option<String>,
    pub is_international: bool,
    pub turnaround_weeks: Option<u16>,
    pub turnaround_label: String,
    pub article_languages: String,
    pub template_url: Option<String>,
    pub risk_warning: Option<String>,
    pub citation_style: Option<String>,
    pub word_limit: Option<(u32, u32)>,
    pub cited_count: u32,
    pub submission_open: Option<bool>,
    #[serde(default)]
    pub submission_fee_idr: Option<u64>,
    #[serde(default)]
    pub bot_protected: bool,
    #[serde(default)]
    pub issn_valid: bool,
    #[serde(default)]
    pub extra_indexes: Vec<String>,
    #[serde(default)]
    pub editor_in_chief: Option<String>,
    #[serde(default)]
    pub max_similarity_pct: Option<u8>,
}

impl From<&JournalRecord> for UiJournalItem {
    fn from(r: &JournalRecord) -> Self {
        let sinta_num = r
            .sinta_rank_num()
            .map(|n| n.to_string())
            .unwrap_or_default();
        let email_short = r
            .email
            .split([',', ';'])
            .map(str::trim)
            .find(|s| !s.is_empty() && s.contains('@'))
            .unwrap_or("Email belum terdata")
            .to_string();
        let submit_label = match r.submission_fee_idr {
            Some(0) => "Gratis (Submission)".to_string(),
            Some(v) if v > 0 => format_idr_short(v as i64),
            _ => "-".to_string(),
        };
        let issn_valid =
            r.issn_valid || crate::parser::heuristics::has_valid_issn_pair(&r.issn, &r.eissn);
        let health_warning = r.is_incomplete
            || ((r.email.trim().is_empty() || r.email.contains("tidak ditemukan"))
                && r.publish_month_nums.is_empty());
        Self {
            nama: r.title.clone(),
            penerbit: if r.publisher.is_empty() {
                "-".to_string()
            } else {
                r.publisher.clone()
            },
            sumber: if r.source_portal.is_empty() {
                "Garuda/SINTA".to_string()
            } else {
                r.source_portal.clone()
            },
            sinta: sinta_num,
            match_score: r.match_score.round(),
            kesesuaian_naskah: if r.match_label.is_empty() {
                "Cukup Sesuai".to_string()
            } else {
                r.match_label.clone()
            },
            match_reasons: r.match_reasons.clone(),
            apc_label: if r.apc_status.is_empty() {
                crate::config::APC_UNKNOWN.to_string()
            } else {
                r.apc_status.clone()
            },
            apc_category: r.apc_category().to_string(),
            submit_label,
            biaya_publikasi: r.publication_fee_idr,
            apc_idr_numeric: r.resolved_apc_idr_numeric(),
            biaya_sort: r.biaya_sort_idr(),
            jadwal_terbit: if r.publish_schedule.is_empty() {
                "-".to_string()
            } else {
                r.publish_schedule.clone()
            },
            terbit_terdekat: if r.next_issue_estimate.is_empty() {
                "-".to_string()
            } else {
                r.next_issue_estimate.clone()
            },
            email: r.email.clone(),
            email_short,
            telepon: r.phone.clone(),
            url: if r.website_url.starts_with("http") {
                r.website_url.clone()
            } else {
                r.garuda_url.clone()
            },
            sinta_url: r.sinta_url.clone(),
            garuda_url: r.garuda_url.clone(),
            issn: r.issn.clone(),
            eissn: r.eissn.clone(),
            focus_scope: r.focus_and_scope.clone(),
            ai_validated: r.ai_validated,
            ai_source: r.ai_source.clone(),
            health_warning,
            last_issue_year: r.last_issue_year,
            activity_status: if r.activity_status.is_empty() {
                if r.bot_protected {
                    "Proteksi Bot (Buka Web)".to_string()
                } else {
                    "Belum Terdeteksi".to_string()
                }
            } else {
                r.activity_status.clone()
            },
            is_dormant: r.is_dormant,
            apc_evidence: r.apc_evidence.clone(),
            quartile: r.quartile.clone(),
            is_international: r.is_international,
            turnaround_weeks: r.turnaround_weeks,
            turnaround_label: r.turnaround_label.clone(),
            article_languages: r.article_languages.clone(),
            template_url: r.template_url.clone(),
            risk_warning: r.risk_warning.clone(),
            citation_style: r.citation_style.clone(),
            word_limit: r.word_limit,
            cited_count: r.cited_count,
            submission_open: r.submission_open,
            submission_fee_idr: r.submission_fee_idr,
            bot_protected: r.bot_protected,
            issn_valid,
            extra_indexes: r.extra_indexes.clone(),
            editor_in_chief: r.editor_in_chief.clone(),
            max_similarity_pct: r.max_similarity_pct,
        }
    }
}

impl From<&UiJournalItem> for JournalRecord {
    fn from(it: &UiJournalItem) -> Self {
        let apc_numeric = it.resolved_apc_idr_numeric();
        let fee_idr = apc_numeric.map(|v| v as i64);
        let website_url = if it.url.trim().is_empty() {
            it.garuda_url.clone()
        } else {
            it.url.clone()
        };
        Self {
            title: it.nama.clone(),
            publisher: if it.penerbit == "-" {
                String::new()
            } else {
                it.penerbit.clone()
            },
            issn: it.issn.clone(),
            eissn: it.eissn.clone(),
            garuda_url: it.garuda_url.clone(),
            website_url,
            sinta_url: it.sinta_url.clone(),
            source_portal: it.sumber.clone(),
            sinta_score: if it.sinta.trim().is_empty() {
                String::new()
            } else {
                format!("Sinta {}", it.sinta.trim())
            },
            focus_and_scope: it.focus_scope.clone(),
            publication_fee: it.apc_label.clone(),
            publication_fee_idr: fee_idr,
            apc_idr_numeric: apc_numeric,
            submission_fee_idr: it.submission_fee_idr,
            apc_status: it.apc_label.clone(),
            apc_evidence: it.apc_evidence.clone(),
            email: it.email.clone(),
            phone: it.telepon.clone(),
            publish_schedule: if it.jadwal_terbit == "-" {
                String::new()
            } else {
                it.jadwal_terbit.clone()
            },
            next_issue_estimate: if it.terbit_terdekat == "-" {
                String::new()
            } else {
                it.terbit_terdekat.clone()
            },
            ai_validated: it.ai_validated,
            ai_source: it.ai_source.clone(),
            match_score: it.match_score,
            match_label: it.kesesuaian_naskah.clone(),
            match_reasons: it.match_reasons.clone(),
            last_issue_year: it.last_issue_year,
            activity_status: it.activity_status.clone(),
            is_dormant: it.is_dormant,
            bot_protected: it.bot_protected,
            issn_valid: it.issn_valid,
            extra_indexes: it.extra_indexes.clone(),
            quartile: it.quartile.clone(),
            is_international: it.is_international,
            turnaround_weeks: it.turnaround_weeks,
            turnaround_label: it.turnaround_label.clone(),
            article_languages: it.article_languages.clone(),
            template_url: it.template_url.clone(),
            risk_warning: it.risk_warning.clone(),
            citation_style: it.citation_style.clone(),
            word_limit: it.word_limit,
            cited_count: it.cited_count,
            submission_open: it.submission_open,
            editor_in_chief: it.editor_in_chief.clone(),
            max_similarity_pct: it.max_similarity_pct,
            ..Default::default()
        }
    }
}

pub fn format_idr_short(amount: i64) -> String {
    let s = amount.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3 + 4);
    for (idx, ch) in s.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    let formatted: String = out.chars().rev().collect();
    format!("Rp {formatted}")
}

pub fn format_usd_short(idr: u64, usd_to_idr: i64) -> String {
    let rate = if usd_to_idr >= 1_000 {
        usd_to_idr as u64
    } else {
        crate::config::USD_TO_IDR_RATE as u64
    };
    let usd = ((idr as f64) / (rate as f64)).round().max(1.0) as u64;
    format!("${usd}")
}

/// Format an APC label cleanly in a single selected currency (`Idr` or `Usd`) with zero mixing.
pub fn format_apc_unified(
    apc_category: &str,
    apc_idr_numeric: Option<u64>,
    fallback_label: &str,
    currency: DisplayCurrency,
    usd_to_idr: i64,
) -> String {
    if apc_category == "free" || apc_idr_numeric == Some(0) {
        return match currency {
            DisplayCurrency::Idr => "Gratis (Rp 0)".to_string(),
            DisplayCurrency::Usd => "Free ($0)".to_string(),
        };
    }
    if let Some(idr) = apc_idr_numeric.filter(|&v| v > 0) {
        return match currency {
            DisplayCurrency::Idr => format_idr_short(idr as i64),
            DisplayCurrency::Usd => format_usd_short(idr, usd_to_idr),
        };
    }
    if fallback_label.trim().is_empty() {
        crate::config::APC_UNKNOWN.to_string()
    } else {
        fallback_label.to_string()
    }
}

impl UiJournalItem {
    /// Resolve pure numeric IDR value without losing precision when switching between IDR & USD.
    pub fn resolved_apc_idr_numeric(&self) -> Option<u64> {
        if self.apc_category == "free" {
            return Some(0);
        }
        self.apc_idr_numeric
            .or_else(|| self.biaya_publikasi.filter(|&v| v >= 0).map(|v| v as u64))
    }

    pub fn formatted_apc(&self, currency: DisplayCurrency, usd_to_idr: i64) -> String {
        format_apc_unified(
            &self.apc_category,
            self.resolved_apc_idr_numeric(),
            &self.apc_label,
            currency,
            usd_to_idr,
        )
    }

    pub fn apply_display_currency(&mut self, currency: DisplayCurrency, usd_to_idr: i64) {
        let numeric = self.resolved_apc_idr_numeric();
        if self.apc_idr_numeric.is_none() {
            self.apc_idr_numeric = numeric;
        }
        if self.biaya_publikasi.is_none() {
            self.biaya_publikasi = numeric.map(|v| v as i64);
        }
        self.apc_label = format_apc_unified(
            &self.apc_category,
            numeric,
            &self.apc_label,
            currency,
            usd_to_idr,
        );
    }
}

pub fn reformat_ui_results_currency(
    items: &mut [UiJournalItem],
    currency: DisplayCurrency,
    usd_to_idr: i64,
) {
    for item in items {
        item.apply_display_currency(currency, usd_to_idr);
    }
}

fn default_true() -> bool {
    true
}

fn default_target() -> usize {
    25
}

fn default_max_rp() -> i64 {
    15_000_000
}

fn default_usd_rate() -> i64 {
    crate::config::USD_TO_IDR_RATE
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub keyword: String,
    #[serde(default = "default_target", alias = "target")]
    pub target_count: usize,
    #[serde(default)]
    pub months: Vec<serde_json::Value>,
    #[serde(default, alias = "sinta")]
    pub sinta_tiers: Vec<serde_json::Value>,
    #[serde(default = "default_true", alias = "ai_enabled")]
    pub use_ai: bool,
    #[serde(default)]
    pub ai_model: Option<String>,
    #[serde(default = "default_true")]
    pub garuda: bool,
    #[serde(default = "default_true")]
    pub sinta_src: bool,
    #[serde(default)]
    pub openalex: bool,
    #[serde(default)]
    pub doaj: bool,
    #[serde(default)]
    pub quartiles: Vec<String>,
    #[serde(default)]
    pub filter_quartiles: bool,
    #[serde(default = "default_usd_rate")]
    pub usd_to_idr: i64,
    #[serde(default)]
    pub display_currency: DisplayCurrency,
    #[serde(default)]
    pub only_free: bool,
    #[serde(default = "default_true")]
    pub include_na: bool,
    #[serde(default = "default_true")]
    pub include_unknown: bool,
    #[serde(default)]
    pub min_rp: i64,
    #[serde(default = "default_max_rp")]
    pub max_rp: i64,
    #[serde(default = "default_true", alias = "match")]
    pub match_enabled: bool,
    #[serde(default)]
    pub include_non_sinta_garuda: bool,
    #[serde(default = "default_true")]
    pub exclude_dormant: bool,
    #[serde(default)]
    pub only_open_submission: bool,
    #[serde(default)]
    pub offline_mode: bool,
    #[serde(default)]
    pub max_turnaround_weeks: Option<u16>,
    #[serde(default)]
    pub target_deadline_month: Option<u8>,
    #[serde(default)]
    pub formats: Vec<String>,
}

impl SearchParams {
    pub fn normalized_keywords(&self) -> Vec<String> {
        let mut out = Vec::new();
        for kw in &self.keywords {
            for part in kw.split(',') {
                let p = part.trim();
                if !p.is_empty() && !out.iter().any(|x: &String| x.eq_ignore_ascii_case(p)) {
                    out.push(p.to_string());
                }
            }
        }
        if !self.keyword.trim().is_empty() {
            for part in self.keyword.split(',') {
                let p = part.trim();
                if !p.is_empty() && !out.iter().any(|x: &String| x.eq_ignore_ascii_case(p)) {
                    out.push(p.to_string());
                }
            }
        }
        out
    }

    pub fn normalized_sinta_ranks(&self) -> Vec<u8> {
        let mut ranks = Vec::new();
        for v in &self.sinta_tiers {
            match v {
                serde_json::Value::Number(n) => {
                    if let Some(u) = n.as_u64() {
                        if (1..=6).contains(&u) {
                            ranks.push(u as u8);
                        }
                    }
                }
                serde_json::Value::String(s) => {
                    for ch in ['1', '2', '3', '4', '5', '6'] {
                        if s.contains(ch) {
                            if let Some(d) = ch.to_digit(10) {
                                ranks.push(d as u8);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        ranks.sort_unstable();
        ranks.dedup();
        ranks
    }

    pub fn normalized_months(&self) -> Vec<u8> {
        let mut months = Vec::new();
        for v in &self.months {
            match v {
                serde_json::Value::Number(n) => {
                    if let Some(u) = n.as_u64() {
                        if (1..=12).contains(&u) {
                            months.push(u as u8);
                        }
                    }
                }
                serde_json::Value::String(s) => {
                    if let Ok(u) = s.trim().parse::<u8>() {
                        if (1..=12).contains(&u) {
                            months.push(u);
                        }
                    } else {
                        for m in crate::parser::month::extract_months_from_text(s) {
                            months.push(m);
                        }
                    }
                }
                _ => {}
            }
        }
        months.sort_unstable();
        months.dedup();
        months
    }

    pub fn effective_target(&self) -> usize {
        self.target_count.clamp(1, crate::config::MAX_TARGET_COUNT)
    }

    pub fn normalized_quartiles(&self) -> Vec<String> {
        let mut qs = Vec::new();
        for raw in &self.quartiles {
            let up = raw.trim().to_ascii_uppercase();
            if matches!(up.as_str(), "Q1" | "Q2" | "Q3" | "Q4") && !qs.contains(&up) {
                qs.push(up);
            }
        }
        qs.sort();
        qs
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub percent: f64,
    pub current_item: String,
    pub eta_seconds: Option<u64>,
    pub status_text: String,
    pub rate_per_sec: f64,
    pub elapsed_seconds: f64,
    pub kpi_total: usize,
    pub kpi_free: usize,
    pub kpi_paid: usize,
    pub kpi_fast: usize,
}

/// Pre-submission IMRAD & reference structure checklist (Feature #10).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreSubmitChecklist {
    pub word_count: usize,
    pub has_abstract: bool,
    pub has_keywords: bool,
    pub has_intro: bool,
    pub has_method: bool,
    pub has_results: bool,
    pub has_references: bool,
    pub estimated_refs: usize,
    pub imrad_complete: bool,
    pub summary_badge: String,
    #[serde(default)]
    pub uncited_long_paragraphs: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManuscriptMeta {
    pub filename: String,
    pub char_count: usize,
    pub extracted_text: String,
    pub detected_keywords: Vec<String>,
    pub detected_domain: String,
    #[serde(default)]
    pub extracted_title: String,
    #[serde(default)]
    pub suggested_keywords: String,
    #[serde(default)]
    pub abstract_snippet: String,
    #[serde(default)]
    pub checklist: PreSubmitChecklist,
    #[serde(default)]
    pub cited_journals: HashMap<String, u32>,
    #[serde(default)]
    pub recommended_topics: Vec<String>,
}

pub type ManuscriptContext = ManuscriptMeta;

#[derive(Debug, Clone, Serialize)]
pub struct AiStatus {
    pub installed: bool,
    pub running: bool,
    pub models: Vec<String>,
    pub recommended_model: String,
    pub circuit_open: bool,
    pub task_running: bool,
    pub task_percent: f64,
    pub task_message: String,
}

#[derive(Debug, Clone, Default)]
pub struct ReadinessReport {
    pub score: u8,
    pub checklist: Vec<(bool, String)>,
}

/// Calculate Pre-Submission Readiness Score (0–100%) and checklist `[✓]`/`[!]`.
pub fn compute_submission_readiness(
    item: &UiJournalItem,
    ms_word_count: Option<usize>,
    ms_imrad_complete: Option<bool>,
    is_en: bool,
) -> ReadinessReport {
    let mut score: f64 = 0.0;
    let mut checklist = Vec::new();

    // 1. Topic similarity (BM25) — max 40 pts
    let topic_pts = (item.match_score.clamp(0.0, 100.0) * 0.40).round();
    score += topic_pts;
    let topic_ok = item.match_score >= 55.0;
    checklist.push((
        topic_ok,
        if is_en {
            format!("Topic Relevance: {:.0}%", item.match_score)
        } else {
            format!("Kecocokan Topik Naskah: {:.0}%", item.match_score)
        },
    ));

    // 2. Word limit FitStatus — max 25 pts
    match ms_word_count {
        Some(wc) => {
            let wc_u32 = wc.min(u32::MAX as usize) as u32;
            match crate::parser::doc_matcher::check_manuscript_word_fit(wc_u32, item.word_limit) {
                Some(FitStatus::Sesuai) => {
                    score += 25.0;
                    checklist.push((
                        true,
                        if is_en {
                            format!("Word count ({wc} words) matches journal limit")
                        } else {
                            format!("Panjang kata ({wc} kata) sesuai syarat jurnal")
                        },
                    ));
                }
                Some(FitStatus::TerlaluPendek) => {
                    score += 8.0;
                    let req = item
                        .word_limit
                        .map(|(a, b)| format!("{a}-{b}"))
                        .unwrap_or_default();
                    checklist.push((
                        false,
                        if is_en {
                            format!("Word count too short ({wc} vs {req} words)")
                        } else {
                            format!("Naskah terlalu pendek ({wc} vs {req} kata)")
                        },
                    ));
                }
                Some(FitStatus::MelebihiBatas) => {
                    score += 8.0;
                    let req = item
                        .word_limit
                        .map(|(_, b)| format!("<= {b}"))
                        .unwrap_or_default();
                    checklist.push((
                        false,
                        if is_en {
                            format!("Word count exceeds limit ({wc} vs {req} words)")
                        } else {
                            format!("Panjang kata melebihi batas ({wc} vs {req} kata)")
                        },
                    ));
                }
                None => {
                    score += 18.0;
                    checklist.push((
                        true,
                        if is_en {
                            format!("Manuscript loaded ({wc} words; no hard journal cap detected)")
                        } else {
                            format!(
                                "Naskah termuat ({wc} kata; jurnal tidak mencantumkan batas ketat)"
                            )
                        },
                    ));
                }
            }
        }
        None => {
            checklist.push((
                false,
                if is_en {
                    "Upload manuscript (.docx/.pdf) to verify word-count fit".to_string()
                } else {
                    "Unggah naskah (.docx/.pdf) untuk validasi batas kata".to_string()
                },
            ));
        }
    }

    // 3. IMRAD structure completeness — max 25 pts
    match ms_imrad_complete {
        Some(true) => {
            score += 25.0;
            checklist.push((
                true,
                if is_en {
                    "IMRAD structure & references complete".to_string()
                } else {
                    "Struktur IMRAD & daftar pustaka lengkap".to_string()
                },
            ));
        }
        Some(false) => {
            score += 10.0;
            checklist.push((
                false,
                if is_en {
                    "IMRAD structure incomplete (review Introduction/Method/Results/References)"
                        .to_string()
                } else {
                    "Struktur IMRAD belum lengkap (cek Pendahuluan/Metode/Hasil/Pustaka)"
                        .to_string()
                },
            ));
        }
        None => {
            checklist.push((
                false,
                if is_en {
                    "Upload manuscript to verify IMRAD structure".to_string()
                } else {
                    "Unggah naskah untuk audit kelengkapan struktur IMRAD".to_string()
                },
            ));
        }
    }

    // 4. Portal activity & submission gate — max 10 pts
    let portal_ok = !item.is_dormant && item.submission_open != Some(false);
    if portal_ok {
        score += 10.0;
    }
    checklist.push((
        portal_ok,
        if item.submission_open == Some(false) {
            if is_en {
                "Journal submission appears closed".to_string()
            } else {
                "Pendaftaran naskah jurnal sedang tutup".to_string()
            }
        } else if item.is_dormant {
            if is_en {
                "Journal appears dormant — verify latest issue before submitting".to_string()
            } else {
                "Jurnal terindikasi dorman — cek arsip terbaru sebelum submit".to_string()
            }
        } else if is_en {
            "Journal portal active / open for submission".to_string()
        } else {
            "Portal jurnal aktif / terbuka untuk pengajuan".to_string()
        },
    ));

    ReadinessReport {
        score: score.clamp(0.0, 100.0).round() as u8,
        checklist,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readiness_score_and_integrity_mapping() {
        let item = UiJournalItem {
            nama: "Jurnal Informatika Nusantara".to_string(),
            match_score: 85.0,
            word_limit: Some((4000, 7000)),
            submission_open: Some(true),
            is_dormant: false,
            submission_fee_idr: Some(150_000),
            apc_idr_numeric: Some(500_000),
            ..Default::default()
        };
        let rep = compute_submission_readiness(&item, Some(5200), Some(true), false);
        assert_eq!(rep.score, 94);
        assert!(rep.checklist.iter().all(|(ok, _)| *ok));

        let rep_short = compute_submission_readiness(&item, Some(2200), Some(false), false);
        assert!(rep_short.score < 70);
        assert!(rep_short.checklist.iter().any(|(ok, _)| !*ok));
    }
}

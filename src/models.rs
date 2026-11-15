use serde::{Deserialize, Serialize};

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
    pub submission_fee_idr: Option<i64>,
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
    pub apc_evidence: String,
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
        let (status, dormant) =
            compute_activity_status(self.last_issue_year, broken_archive, current_year);
        self.activity_status = status;
        self.is_dormant = dormant;
    }
    /// Classify APC into `"free"`, `"paid"`, or `"unknown"` consistently for UI/filter/sort.
    pub fn apc_category(&self) -> &'static str {
        let s = self.apc_status.to_lowercase();
        if self.publication_fee_idr == Some(0)
            || s.contains("gratis")
            || s.contains("rp 0")
            || s.contains("bebas biaya")
        {
            return "free";
        }
        if self.publication_fee_idr.is_some_and(|v| v > 0) {
            return "paid";
        }
        let f = self.fee_amount.trim();
        if !f.is_empty() && f.parse::<f64>().is_ok_and(|v| v > 0.0) {
            return "paid";
        }
        "unknown"
    }

    /// Extract SINTA numeric rank 1..=6 if available.
    pub fn sinta_rank_num(&self) -> Option<u8> {
        let up = self.sinta_score.to_uppercase();
        for ch in ['1', '2', '3', '4', '5', '6'] {
            if up.contains(&format!("S{ch}")) || up.contains(&format!("SINTA {ch}")) || up.trim() == ch.to_string() {
                return ch.to_digit(10).map(|d| d as u8);
            }
        }
        None
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
#[derive(Debug, Clone, Default, Serialize)]
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
            Some(v) if v > 0 => format_idr_short(v),
            _ => "-".to_string(),
        };
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
                "Belum Terdeteksi".to_string()
            } else {
                r.activity_status.clone()
            },
            is_dormant: r.is_dormant,
            apc_evidence: r.apc_evidence.clone(),
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

fn default_true() -> bool {
    true
}

fn default_target() -> usize {
    25
}

fn default_max_rp() -> i64 {
    15_000_000
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManuscriptMeta {
    pub filename: String,
    pub char_count: usize,
    pub extracted_text: String,
    pub detected_keywords: Vec<String>,
    pub detected_domain: String,
}

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

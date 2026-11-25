//! Bilingual ID / EN localization dictionary and persisted `.cache/app_settings.json` preferences.

pub use crate::models::DisplayCurrency;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AppLanguage {
    #[default]
    Id,
    En,
}

impl AppLanguage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Id => "Bahasa Indonesia",
            Self::En => "English",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Id => Self::En,
            Self::En => Self::Id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppSettings {
    pub language: AppLanguage,
    pub dark_theme: bool,
    pub default_garuda: bool,
    pub default_sinta: bool,
    pub default_openalex: bool,
    pub default_doaj: bool,
    pub usd_to_idr: i64,
    pub display_currency: DisplayCurrency,
    pub polite_bot_mode: bool,
    pub author_name: String,
    pub author_id: String,
    pub author_affiliation: String,
    pub author_email: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: AppLanguage::Id,
            dark_theme: true,
            default_garuda: true,
            default_sinta: true,
            default_openalex: true,
            default_doaj: true,
            usd_to_idr: crate::config::USD_TO_IDR_RATE,
            display_currency: DisplayCurrency::Idr,
            polite_bot_mode: false,
            author_name: String::new(),
            author_id: String::new(),
            author_affiliation: String::new(),
            author_email: String::new(),
        }
    }
}

impl AppSettings {
    pub fn normalized(mut self) -> Self {
        if self.usd_to_idr < 1_000 {
            self.usd_to_idr = crate::config::USD_TO_IDR_RATE;
        }
        self.author_name = self.author_name.trim().to_string();
        self.author_id = self.author_id.trim().to_string();
        self.author_affiliation = self.author_affiliation.trim().to_string();
        self.author_email = self.author_email.trim().to_string();
        self
    }
}

pub fn load_app_settings_from(path: &Path) -> AppSettings {
    crate::config::load_json_or_default::<AppSettings>(path).normalized()
}

pub fn save_app_settings_to(path: &Path, settings: &AppSettings) -> std::io::Result<()> {
    crate::config::save_json_pretty(path, &settings.clone().normalized())
}

pub fn load_app_settings() -> AppSettings {
    load_app_settings_from(&crate::config::app_settings_path())
}

pub fn save_app_settings(settings: &AppSettings) -> std::io::Result<()> {
    save_app_settings_to(&crate::config::app_settings_path(), settings)
}

#[derive(Debug, Clone, Copy)]
pub struct Texts {
    // Sidebar
    pub nav_search: &'static str,
    pub nav_detail: &'static str,
    pub nav_dospen: &'static str,
    pub nav_filters: &'static str,
    pub nav_match: &'static str,
    pub nav_history: &'static str,
    pub nav_settings: &'static str,
    pub nav_guide: &'static str,
    pub status_searching: &'static str,
    pub theme_change: &'static str,

    // Search Page
    pub search_title: &'static str,
    pub keyword_hint: &'static str,
    pub target_label: &'static str,
    pub upload_ms: &'static str,
    pub remove_ms: &'static str,
    pub start_search: &'static str,
    pub stop_search: &'static str,
    pub filter_btn: &'static str,
    pub preset_label: &'static str,
    pub preset_skripsi: &'static str,
    pub preset_free: &'static str,
    pub preset_soon: &'static str,
    pub preset_high: &'static str,
    pub kpi_total: &'static str,
    pub kpi_free: &'static str,
    pub kpi_paid: &'static str,
    pub kpi_soon: &'static str,
    pub results_heading: &'static str,
    pub results_unit: &'static str,
    pub shortlisted_btn: &'static str,
    pub copy_wa_btn: &'static str,
    pub filter_table_hint: &'static str,
    pub col_journal: &'static str,
    pub col_accred: &'static str,
    pub col_match: &'static str,
    pub col_apc: &'static str,
    pub col_activity: &'static str,
    pub col_action: &'static str,

    // Filters Page
    pub filters_title: &'static str,
    pub filters_subtitle: &'static str,
    pub filters_card1: &'static str,
    pub filters_intl_section: &'static str,
    pub filters_intl_toggle: &'static str,
    pub filters_intl_hint: &'static str,
    pub filters_months_card: &'static str,
    pub filters_budget_card: &'static str,
    pub filters_ai_card: &'static str,
    pub filters_reset_btn: &'static str,

    // History & Export Page
    pub history_title: &'static str,
    pub history_subtitle: &'static str,
    pub export_card_title: &'static str,
    pub export_ready_suffix: &'static str,
    pub sub_sec_1: &'static str,
    pub sub_sec_2: &'static str,
    pub sub_sec_3: &'static str,
    pub btn_export_xlsx: &'static str,
    pub btn_export_md: &'static str,
    pub btn_export_csv: &'static str,
    pub btn_open_last_xlsx: &'static str,
    pub btn_copy_wa: &'static str,
    pub btn_copy_emails: &'static str,
    pub btn_copy_draft: &'static str,
    pub btn_copy_dosbing: &'static str,
    pub history_card_title: &'static str,
    pub sessions_saved_suffix: &'static str,
    pub load_results_btn: &'static str,
    pub apply_filter_btn: &'static str,
    pub folder_cache_card: &'static str,
    pub logs_card_title: &'static str,
    pub show_full_log: &'static str,
    pub hide_full_log: &'static str,

    // Settings Page
    pub settings_title: &'static str,
    pub settings_subtitle: &'static str,
    pub settings_lang_card: &'static str,
    pub settings_lang_label: &'static str,
    pub settings_lang_hint: &'static str,
    pub settings_theme_label: &'static str,
    pub settings_theme_hint: &'static str,
    pub settings_portal_card: &'static str,
    pub settings_currency_label: &'static str,
    pub settings_currency_hint: &'static str,
    pub settings_usd_label: &'static str,
    pub settings_usd_hint: &'static str,
    pub settings_save_btn: &'static str,
    pub settings_reset_btn: &'static str,

    // Match Engine & Guide Titles
    pub match_title: &'static str,
    pub match_subtitle: &'static str,
    pub guide_title: &'static str,
    pub guide_subtitle: &'static str,

    // Common Toasts
    pub toast_empty_kw: &'static str,
    pub toast_no_portal: &'static str,
    pub toast_stopping: &'static str,
    pub toast_settings_saved: &'static str,
    pub toast_settings_reset: &'static str,
    pub toast_lang_switched: &'static str,
}

const ID_TEXTS: Texts = Texts {
    nav_search: "1. Cari Jurnal",
    nav_detail: "2. Detail & Naskah",
    nav_dospen: "3. Shortlist & Dospen",
    nav_filters: "Filter Lanjutan",
    nav_match: "Match Engine & AI",
    nav_history: "Riwayat & Ekspor",
    nav_settings: "Pengaturan",
    nav_guide: "Panduan & Tentang",
    status_searching: "Mencari...",
    theme_change: "Ubah",

    search_title: "Pencarian Jurnal SINTA, Garuda & Q1–Q4",
    keyword_hint: "Ketik topik skripsi / riset (mis. sistem informasi, akuntansi, UMKM)...",
    target_label: "Target:",
    upload_ms: "Unggah Naskah",
    remove_ms: "Hapus Naskah",
    start_search: "Mulai Cari Jurnal",
    stop_search: "Hentikan",
    filter_btn: "Filter",
    preset_label: "Preset:",
    preset_skripsi: "Target Skripsi S1",
    preset_free: "Jurnal Gratis (Rp 0)",
    preset_soon: "Terbit Terdekat",
    preset_high: "Reputasi Tinggi",
    kpi_total: "Total Jurnal",
    kpi_free: "Gratis Rp 0",
    kpi_paid: "Berbayar",
    kpi_soon: "Terbit 1-3 Bln",
    results_heading: "Hasil Pencarian",
    results_unit: "jurnal",
    shortlisted_btn: "Pilihan",
    copy_wa_btn: "Salin WA",
    filter_table_hint: "Filter nama, penerbit, email...",
    col_journal: "Jurnal & Penerbit",
    col_accred: "SINTA / Q",
    col_match: "Match",
    col_apc: "Biaya Publikasi",
    col_activity: "Keaktifan & Terbit",
    col_action: "Aksi",

    filters_title: "Filter Lanjutan",
    filters_subtitle:
        "Kendalikan peringkat SINTA, kuartil Q1–Q4, portal internasional, bulan terbit, anggaran APC, dan ekspor.",
    filters_card1: "Peringkat SINTA, Kuartil Q1–Q4 & Portal Sumber",
    filters_intl_section: "Kuartil Internasional (OpenAlex & DOAJ):",
    filters_intl_toggle: "Portal Internasional (OpenAlex & DOAJ)",
    filters_intl_hint:
        "Cari jurnal internasional Q1–Q4 & Diamond OA resmi tanpa blokir Cloudflare.",
    filters_months_card: "Jadwal Bulan Terbit (1–12)",
    filters_budget_card: "Anggaran APC & Audit Keaktifan",
    filters_ai_card: "Validasi Cerdas & Format Ekspor",
    filters_reset_btn: "Reset Filter Default",

    history_title: "Riwayat & Ekspor",
    history_subtitle:
        "Ekspor selektif, salin daftar WhatsApp & email redaksi, serta muat ulang riwayat sesi & filter.",
    export_card_title: "Berbagi & Ekspor Selektif",
    export_ready_suffix: "jurnal siap ekspor",
    sub_sec_1: "1. Pilih Data yang Akan Diekspor",
    sub_sec_2: "2. Simpan & Buka File Laporan",
    sub_sec_3: "3. Salin Cepat ke Clipboard (WhatsApp & Email)",
    btn_export_xlsx: "Ekspor Excel (.xlsx)",
    btn_export_md: "Ekspor Markdown (.md)",
    btn_export_csv: "Ekspor CSV (.csv)",
    btn_open_last_xlsx: "Buka File Excel Terakhir",
    btn_copy_wa: "Salin Daftar WA",
    btn_copy_emails: "Salin Email Redaksi",
    btn_copy_draft: "Salin Draf Chat Editor",
    btn_copy_dosbing: "Salin Ringkasan Dosbing",
    history_card_title: "Riwayat Pencarian & Filter",
    sessions_saved_suffix: "sesi tersimpan",
    load_results_btn: "Muat Hasil",
    apply_filter_btn: "Terapkan Filter",
    folder_cache_card: "Folder Output & Cache SQLite",
    logs_card_title: "Log Aktivitas & Diagnostik",
    show_full_log: "Tampilkan Log Lengkap",
    hide_full_log: "Sembunyikan Log",

    settings_title: "Pengaturan & Bahasa",
    settings_subtitle:
        "Sesuaikan bahasa antarmuka, tema, portal default (SINTA, Garuda, OpenAlex, DOAJ), dan kurs USD.",
    settings_lang_card: "Bahasa & Tampilan",
    settings_lang_label: "Bahasa Antarmuka (UI Language)",
    settings_lang_hint:
        "Mengganti seluruh teks menu, tombol, filter, dan notifikasi secara instan.",
    settings_theme_label: "Tema Aplikasi",
    settings_theme_hint: "Pilih mode gelap (JSFS Dark) atau terang kontras tinggi (JSFS Daylight).",
    settings_portal_card: "Sumber Portal Default & Kurs Mata Uang",
    settings_currency_label: "Mata Uang Biaya APC",
    settings_currency_hint:
        "Seragamkan tampilan biaya APC di tabel & ekspor dalam Rupiah (IDR) atau Dollar (USD).",
    settings_usd_label: "Kurs Konversi 1 USD ke IDR",
    settings_usd_hint: "Dipakai untuk mengestimasi biaya APC jurnal internasional (default Rp 16.500).",
    settings_save_btn: "Simpan Pengaturan",
    settings_reset_btn: "Reset Default",

    match_title: "Match Engine & Validasi AI",
    match_subtitle: "Pilih mesin pencocokan topik naskah & validasi Focus-Scope / APC jurnal.",
    guide_title: "Panduan & Tentang JSFS",
    guide_subtitle:
        "Panduan ringkas mencari jurnal nasional SINTA & internasional Q1–Q4 untuk publikasi mahasiswa.",

    toast_empty_kw: "Ketik topik atau kata kunci penelitian terlebih dahulu.",
    toast_no_portal: "Aktifkan minimal satu portal (Garuda / SINTA / OpenAlex / DOAJ).",
    toast_stopping: "Menghentikan pencarian...",
    toast_settings_saved: "Pengaturan disimpan ke .cache/app_settings.json.",
    toast_settings_reset: "Pengaturan dikembalikan ke standar awal.",
    toast_lang_switched: "Bahasa diubah ke Bahasa Indonesia.",
};

const EN_TEXTS: Texts = Texts {
    nav_search: "1. Search Journals",
    nav_detail: "2. Journal & Paper",
    nav_dospen: "3. Shortlist & Advisor",
    nav_filters: "Advanced Filters",
    nav_match: "Match Engine & AI",
    nav_history: "History & Export",
    nav_settings: "Settings",
    nav_guide: "Guide & About",
    status_searching: "Searching...",
    theme_change: "Switch",

    search_title: "SINTA, Garuda & Q1–Q4 Journal Search",
    keyword_hint: "Type thesis / research topic (e.g. information systems, accounting, SME)...",
    target_label: "Target:",
    upload_ms: "Upload Manuscript",
    remove_ms: "Remove File",
    start_search: "Start Search",
    stop_search: "Stop",
    filter_btn: "Filters",
    preset_label: "Preset:",
    preset_skripsi: "Undergrad Thesis S1",
    preset_free: "Free Journals ($0)",
    preset_soon: "Upcoming Issues",
    preset_high: "High Reputation",
    kpi_total: "Total Journals",
    kpi_free: "Free (Rp 0)",
    kpi_paid: "Paid APC",
    kpi_soon: "Next 1-3 Mo",
    results_heading: "Search Results",
    results_unit: "journals",
    shortlisted_btn: "Shortlist",
    copy_wa_btn: "Copy WA",
    filter_table_hint: "Filter title, publisher, email...",
    col_journal: "Journal & Publisher",
    col_accred: "SINTA / Q",
    col_match: "Match",
    col_apc: "Publication Fee",
    col_activity: "Activity & Issue",
    col_action: "Actions",

    filters_title: "Advanced Filters",
    filters_subtitle:
        "Control SINTA ranks, Q1–Q4 quartiles, international portals, publish months, APC budget, and exports.",
    filters_card1: "SINTA Ranks, Q1–Q4 Quartiles & Source Portals",
    filters_intl_section: "International Quartiles (OpenAlex & DOAJ):",
    filters_intl_toggle: "International Portals (OpenAlex & DOAJ)",
    filters_intl_hint:
        "Discover official Q1–Q4 & Diamond OA international journals without Cloudflare blocks.",
    filters_months_card: "Publish Schedule Months (1–12)",
    filters_budget_card: "APC Budget & Activity Quality Gate",
    filters_ai_card: "Smart Validation & Export Formats",
    filters_reset_btn: "Reset Default Filters",

    history_title: "History & Export",
    history_subtitle:
        "Targeted exports, WhatsApp & editorial email copy, and saved search/filter session reload.",
    export_card_title: "Targeted Sharing & Export",
    export_ready_suffix: "journals ready",
    sub_sec_1: "1. Choose Data to Export",
    sub_sec_2: "2. Save & Open Report Files",
    sub_sec_3: "3. Quick Copy to Clipboard (WhatsApp & Email)",
    btn_export_xlsx: "Export Excel (.xlsx)",
    btn_export_md: "Export Markdown (.md)",
    btn_export_csv: "Export CSV (.csv)",
    btn_open_last_xlsx: "Open Last Excel File",
    btn_copy_wa: "Copy WA List",
    btn_copy_emails: "Copy Editorial Emails",
    btn_copy_draft: "Copy Editor Inquiry Draft",
    btn_copy_dosbing: "Copy Supervisor Brief",
    history_card_title: "Search & Filter History",
    sessions_saved_suffix: "saved sessions",
    load_results_btn: "Load Results",
    apply_filter_btn: "Apply Filter",
    folder_cache_card: "Output Folder & SQLite Cache",
    logs_card_title: "Activity & Diagnostic Log",
    show_full_log: "Show Full Log",
    hide_full_log: "Hide Log",

    settings_title: "Settings & Language",
    settings_subtitle:
        "Customize UI language, theme, default portals (SINTA, Garuda, OpenAlex, DOAJ), and USD rate.",
    settings_lang_card: "Language & Appearance",
    settings_lang_label: "Interface Language",
    settings_lang_hint:
        "Instantly translates sidebar items, headings, filters, action buttons, and toasts.",
    settings_theme_label: "Application Theme",
    settings_theme_hint: "Choose dark mode (JSFS Dark) or high-contrast daylight (JSFS Daylight).",
    settings_portal_card: "Default Portal Sources & Currency Rate",
    settings_currency_label: "APC Display Currency",
    settings_currency_hint:
        "Unify all APC fees across the table & exports in either Rupiah (IDR) or Dollar (USD).",
    settings_usd_label: "1 USD to IDR Exchange Rate",
    settings_usd_hint:
        "Used to estimate Rupiah cost for international APC journals (default Rp 16,500).",
    settings_save_btn: "Save Settings",
    settings_reset_btn: "Reset Defaults",

    match_title: "Match Engine & AI Validation",
    match_subtitle: "Choose manuscript topic matching engine & Focus-Scope / APC validation.",
    guide_title: "Guide & About JSFS",
    guide_subtitle:
        "Quick guide to finding SINTA national and Q1–Q4 international journals for student publishing.",

    toast_empty_kw: "Please enter a research topic or keyword first.",
    toast_no_portal: "Enable at least one source portal (Garuda / SINTA / OpenAlex / DOAJ).",
    toast_stopping: "Stopping search...",
    toast_settings_saved: "Settings saved to .cache/app_settings.json.",
    toast_settings_reset: "Settings reset to factory defaults.",
    toast_lang_switched: "Language switched to English.",
};

pub fn t(lang: AppLanguage) -> &'static Texts {
    match lang {
        AppLanguage::Id => &ID_TEXTS,
        AppLanguage::En => &EN_TEXTS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_switch_and_app_settings_persistence() {
        assert_eq!(t(AppLanguage::Id).nav_search, "1. Cari Jurnal");
        assert_eq!(t(AppLanguage::En).nav_search, "1. Search Journals");
        assert_eq!(t(AppLanguage::Id).nav_settings, "Pengaturan");
        assert_eq!(t(AppLanguage::En).nav_settings, "Settings");
        assert_eq!(AppLanguage::Id.toggled(), AppLanguage::En);
        assert_eq!(AppLanguage::En.toggled(), AppLanguage::Id);

        let dir = std::env::temp_dir().join("jsfs_v43_settings_test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join(".cache").join("app_settings.json");

        let cfg = AppSettings {
            language: AppLanguage::En,
            dark_theme: false,
            default_garuda: true,
            default_sinta: true,
            default_openalex: true,
            default_doaj: false,
            usd_to_idr: 16_800,
            display_currency: DisplayCurrency::Usd,
            polite_bot_mode: true,
            author_name: "Ahmad Mahasiswa".to_string(),
            author_id: "210123456".to_string(),
            author_affiliation: "Universitas Nusantara".to_string(),
            author_email: "ahmad@campus.ac.id".to_string(),
        };
        save_app_settings_to(&path, &cfg).unwrap();
        let loaded = load_app_settings_from(&path);
        assert_eq!(loaded, cfg);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

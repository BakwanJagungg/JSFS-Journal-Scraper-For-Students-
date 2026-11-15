//! Student 3-step quick guide and About card for JSFS.

use crate::config::APP_VERSION;
use crate::engine::file_ops::open_url_with_feedback;
use crate::engine::EventSink;
use crate::ui::theme::{accent_button, card, card_divider, page_header, pill_badge, Palette};
use egui::{RichText, ScrollArea, Ui};

const REPO_URL: &str = "https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-";

pub fn render_guide_page(ui: &mut Ui, p: &Palette, sink: &EventSink) {
    page_header(
        ui,
        p,
        "Panduan & Tentang JSFS",
        "Panduan ringkas mencari jurnal terakreditasi yang tepat untuk tugas akhir dan publikasi mahasiswa.",
    );

    ScrollArea::vertical().show(ui, |ui| {
        card(ui, p, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("JSFS v{APP_VERSION} - Pure Rust Native Edition"))
                        .size(15.5)
                        .strong()
                        .color(p.text),
                );
                pill_badge(ui, "Pure Rust", p.accent, p.accent_soft);
                pill_badge(ui, "Zero WebView2", p.good, p.good_soft);
                pill_badge(ui, "GPU Rendered", p.warn, p.warn_soft);
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "JSFS memadukan mesin perayap paralel asinkron (SINTA + Garuda + Deep OJS Crawler) dengan antarmuka desktop GPU native yang ringan, cepat, dan siap langsung dipakai.",
                )
                .size(12.5)
                .color(p.muted),
            );
            ui.add_space(10.0);
            if accent_button(ui, p, "Buka Repositori GitHub JSFS").clicked() {
                open_url_with_feedback(REPO_URL, sink);
            }
        });

        ui.add_space(12.0);

        card(ui, p, |ui| {
            ui.label(
                RichText::new("3 Langkah Cepat Mencari Jurnal Mahasiswa")
                    .size(14.0)
                    .strong()
                    .color(p.text),
            );

            for (num, title, body) in [
                (
                    "1",
                    "Masukkan Topik Skripsi atau Unggah Naskah (.docx / .pdf)",
                    "Tulis kata kunci spesifik (pisahkan dengan koma bila lebih dari satu), atur target antara 1 sampai 1.000 jurnal, atau unggah draf naskah agar JSFS menghitung Match Score secara bilingual otomatis.",
                ),
                (
                    "2",
                    "Pilih Preset Mahasiswa & Atur Filter Akurasi",
                    "Gunakan 4 preset instan (Skripsi S1, Jurnal Gratis Rp 0, Terbit Terdekat, Reputasi Tinggi). Aktifkan filter Sembunyikan Jurnal Dorman agar jurnal yang sudah vakum >2 tahun otomatis disaring.",
                ),
                (
                    "3",
                    "Tinjau Bukti APC & Hubungi Redaksi Editor",
                    "Klik baris jurnal untuk memeriksa kutipan teks APC (Bukti APC), tahun edisi terakhir, Focus & Scope bersih, serta gunakan tombol 1-klik Buka Web OJS atau Salin Draf Pesan WA.",
                ),
            ] {
                card_divider(ui, p);
                ui.horizontal_wrapped(|ui| {
                    pill_badge(ui, &format!("Langkah {num}"), p.accent, p.accent_soft);
                    ui.label(RichText::new(title).size(13.0).strong().color(p.text));
                });
                ui.add_space(2.0);
                ui.label(RichText::new(body).size(12.0).color(p.muted));
            }
        });
    });
}

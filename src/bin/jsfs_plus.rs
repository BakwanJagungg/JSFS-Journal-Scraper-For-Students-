#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() -> eframe::Result<()> {
    jsfs::app_entry::run_desktop_app()
}

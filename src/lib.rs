//! JSFS+ Workstation / JSFS Standard: Pure Rust Native Desktop Journal Scraper For Students.

pub mod ai;
pub mod app_entry;
pub mod cache;
pub mod config;
pub mod engine;
pub mod error;
pub mod exporter;
pub mod models;
pub mod parser;
#[cfg(feature = "plus")]
pub mod plus;
pub mod scraper;
pub mod ui;

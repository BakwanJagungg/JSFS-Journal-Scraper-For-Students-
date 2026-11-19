use crate::config::{cache_db_path, CACHE_TTL_DAYS, CACHE_VERSION, SQLITE_CACHE_SIZE_KIB};
use crate::error::JsfsError;
use crate::models::JournalRecord;
use crate::scraper::dedup;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Pair of HTTP conditional validation headers: (ETag, Last-Modified)
pub type HttpHeaderPair = (Option<String>, Option<String>);

/// Local submission tracking entry per journal (Feature #3).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmissionTrack {
    pub journal_key: String,
    pub status: String,
    pub submitted_date: String,
    pub notes: String,
    pub updated_at: String,
}

impl SubmissionTrack {
    pub fn canonical_key(url: &str, title: &str) -> String {
        let norm_url = JournalCache::normalize_url(url);
        if !norm_url.is_empty() && norm_url != "-" {
            norm_url
        } else {
            format!("title::{}", title.trim().to_ascii_lowercase())
        }
    }
}

/// SQLite WAL cache engine with automatic version-bump purging, 14-day TTL, and incomplete bypass.
pub struct JournalCache {
    conn: Connection,
}

impl JournalCache {
    pub fn open_default() -> Result<Self, JsfsError> {
        let path = cache_db_path();
        Self::open_at(&path)
    }

    pub fn open_at(path: &Path) -> Result<Self, JsfsError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        let cache = Self { conn };
        cache.init_schema()?;
        Ok(cache)
    }

    pub fn open_in_memory() -> Result<Self, JsfsError> {
        let conn = Connection::open_in_memory()?;
        let cache = Self { conn };
        cache.init_schema()?;
        Ok(cache)
    }

    fn init_schema(&self) -> Result<(), JsfsError> {
        self.conn
            .execute_batch(&format!("PRAGMA cache_size = -{SQLITE_CACHE_SIZE_KIB};"))?;
        self.conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS journal_cache (
                url TEXT PRIMARY KEY,
                cache_version TEXT NOT NULL,
                fetched_at INTEGER NOT NULL,
                is_incomplete INTEGER NOT NULL DEFAULT 0,
                payload_json TEXT NOT NULL,
                etag TEXT,
                last_modified TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_cache_ver_time
                ON journal_cache(cache_version, fetched_at);
            CREATE TABLE IF NOT EXISTS submission_tracker (
                journal_key TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                submitted_date TEXT NOT NULL DEFAULT '',
                notes TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT ''
            );
            ",
        )?;

        // Migration guard for preexisting tables lacking etag or last_modified columns
        let has_etag: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('journal_cache') WHERE name = 'etag'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_etag {
            let _ = self
                .conn
                .execute_batch("ALTER TABLE journal_cache ADD COLUMN etag TEXT;");
        }
        let has_last_mod: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('journal_cache') WHERE name = 'last_modified'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_last_mod {
            let _ = self
                .conn
                .execute_batch("ALTER TABLE journal_cache ADD COLUMN last_modified TEXT;");
        }

        // Auto-purge stale entries from older cache schema versions
        self.purge_stale_versions(CACHE_VERSION)?;
        Ok(())
    }

    pub fn purge_stale_versions(&self, expected_version: &str) -> Result<usize, JsfsError> {
        let deleted = self.conn.execute(
            "DELETE FROM journal_cache WHERE cache_version != ?1",
            params![expected_version],
        )?;
        Ok(deleted)
    }

    pub fn normalize_url(url: &str) -> String {
        let mut s = url.trim().trim_end_matches('/').to_lowercase();
        if let Some(stripped) = s.strip_suffix("/index") {
            s = stripped.to_string();
        }
        s
    }

    pub fn get(&self, url: &str) -> Result<Option<JournalRecord>, JsfsError> {
        let key = Self::normalize_url(url);
        if key.is_empty() {
            return Ok(None);
        }
        let cutoff = Utc::now().timestamp() - (CACHE_TTL_DAYS * 86_400);
        let row: Option<(String, i64, i64, String)> = self
            .conn
            .query_row(
                "SELECT cache_version, fetched_at, is_incomplete, payload_json
                 FROM journal_cache WHERE url = ?1",
                params![key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;

        match row {
            Some((ver, fetched_at, is_incomplete, payload_json)) => {
                if ver != CACHE_VERSION || fetched_at < cutoff || is_incomplete != 0 {
                    return Ok(None);
                }
                match serde_json::from_str::<JournalRecord>(&payload_json) {
                    Ok(mut rec) => {
                        rec.issn_valid =
                            crate::parser::heuristics::has_valid_issn_pair(&rec.issn, &rec.eissn);
                        Ok(Some(rec))
                    }
                    Err(_) => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    pub fn put(&self, url: &str, record: &JournalRecord) -> Result<(), JsfsError> {
        self.put_with_headers(url, record, None, None)
    }

    /// Insert or update cache entry alongside HTTP conditional caching metadata (ETag & Last-Modified).
    pub fn put_with_headers(
        &self,
        url: &str,
        record: &JournalRecord,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<(), JsfsError> {
        let key = Self::normalize_url(url);
        if key.is_empty() {
            return Ok(());
        }
        let now = Utc::now().timestamp();
        let json = serde_json::to_string(record).unwrap_or_default();
        self.conn.execute(
            "INSERT INTO journal_cache (url, cache_version, fetched_at, is_incomplete, payload_json, etag, last_modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(url) DO UPDATE SET
               cache_version = excluded.cache_version,
               fetched_at = excluded.fetched_at,
               is_incomplete = excluded.is_incomplete,
               payload_json = excluded.payload_json,
               etag = COALESCE(excluded.etag, journal_cache.etag),
               last_modified = COALESCE(excluded.last_modified, journal_cache.last_modified)",
            params![
                key,
                CACHE_VERSION,
                now,
                i32::from(record.is_incomplete),
                json,
                etag,
                last_modified
            ],
        )?;
        Ok(())
    }

    /// Retrieve stored ETag and Last-Modified headers for conditional re-fetching (304 Not Modified).
    pub fn get_conditional_headers(&self, url: &str) -> Result<Option<HttpHeaderPair>, JsfsError> {
        let key = Self::normalize_url(url);
        if key.is_empty() {
            return Ok(None);
        }
        let row: Option<HttpHeaderPair> = self
            .conn
            .query_row(
                "SELECT etag, last_modified FROM journal_cache WHERE url = ?1",
                params![key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    /// Update `fetched_at` timestamp when server returns HTTP 304 Not Modified to keep entry fresh.
    pub fn touch_entry(&self, url: &str) -> Result<bool, JsfsError> {
        let key = Self::normalize_url(url);
        if key.is_empty() {
            return Ok(false);
        }
        let now = Utc::now().timestamp();
        let updated = self.conn.execute(
            "UPDATE journal_cache SET fetched_at = ?1 WHERE url = ?2",
            params![now, key],
        )?;
        Ok(updated > 0)
    }

    pub fn clear(&self) -> Result<usize, JsfsError> {
        let count = self.conn.execute("DELETE FROM journal_cache", [])?;
        Ok(count)
    }

    pub fn count_valid(&self) -> Result<usize, JsfsError> {
        let cutoff = Utc::now().timestamp() - (CACHE_TTL_DAYS * 86_400);
        let cnt: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM journal_cache
             WHERE cache_version = ?1 AND fetched_at >= ?2 AND is_incomplete = 0",
            params![CACHE_VERSION, cutoff],
            |r| r.get(0),
        )?;
        Ok(cnt.max(0) as usize)
    }

    pub fn checkpoint_wal(&self) {
        let _ = self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }

    /// Search local SQLite cache for journals matching `keyword` (Feature #7: Instant Offline Search).
    /// When `keyword` is empty or whitespace, returns up to `limit` most recently cached valid journals.
    pub fn search_cached_journals(
        &self,
        keyword: &str,
        limit: usize,
    ) -> Result<Vec<JournalRecord>, JsfsError> {
        let max_rows = limit.clamp(1, 2000);
        let tokens: Vec<String> = keyword
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| s.len() >= 2)
            .collect();

        let mut stmt = self.conn.prepare(
            "SELECT payload_json FROM journal_cache
             WHERE cache_version = ?1 AND is_incomplete = 0
             ORDER BY fetched_at DESC
             LIMIT 3000",
        )?;
        let rows = stmt.query_map(params![CACHE_VERSION], |r| r.get::<_, String>(0))?;

        let mut matched = Vec::new();
        for payload in rows.flatten() {
            if let Ok(rec) = serde_json::from_str::<JournalRecord>(&payload) {
                if tokens.is_empty() {
                    matched.push(rec);
                } else {
                    let haystack = format!(
                        "{} {} {} {} {} {} {}",
                        rec.title.to_ascii_lowercase(),
                        rec.publisher.to_ascii_lowercase(),
                        rec.subject_area.to_ascii_lowercase(),
                        rec.focus_and_scope.to_ascii_lowercase(),
                        rec.sinta_score.to_ascii_lowercase(),
                        rec.quartile
                            .as_deref()
                            .unwrap_or_default()
                            .to_ascii_lowercase(),
                        rec.issn.to_ascii_lowercase()
                    );
                    if tokens.iter().any(|tok| haystack.contains(tok)) {
                        matched.push(rec);
                    }
                }
            }
        }

        let mut deduped = dedup::deduplicate_and_merge(matched);
        deduped.truncate(max_rows);
        Ok(deduped)
    }

    /// Upsert a submission tracking record for a journal (Feature #3).
    pub fn upsert_submission_track(&self, track: &SubmissionTrack) -> Result<(), JsfsError> {
        if track.journal_key.trim().is_empty() {
            return Ok(());
        }
        let updated_at = if track.updated_at.trim().is_empty() {
            Utc::now().format("%Y-%m-%d %H:%M").to_string()
        } else {
            track.updated_at.clone()
        };
        self.conn.execute(
            "INSERT INTO submission_tracker (journal_key, status, submitted_date, notes, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(journal_key) DO UPDATE SET
               status = excluded.status,
               submitted_date = excluded.submitted_date,
               notes = excluded.notes,
               updated_at = excluded.updated_at",
            params![
                track.journal_key.trim(),
                track.status.trim(),
                track.submitted_date.trim(),
                track.notes.trim(),
                updated_at,
            ],
        )?;
        Ok(())
    }

    /// Retrieve submission tracking record for a single journal key.
    pub fn get_submission_track(
        &self,
        journal_key: &str,
    ) -> Result<Option<SubmissionTrack>, JsfsError> {
        let key = journal_key.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let row = self
            .conn
            .query_row(
                "SELECT journal_key, status, submitted_date, notes, updated_at
                 FROM submission_tracker WHERE journal_key = ?1",
                params![key],
                |r| {
                    Ok(SubmissionTrack {
                        journal_key: r.get(0)?,
                        status: r.get(1)?,
                        submitted_date: r.get(2)?,
                        notes: r.get(3)?,
                        updated_at: r.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Load all submission tracking records mapped by `journal_key`.
    pub fn load_all_submission_tracks(
        &self,
    ) -> Result<HashMap<String, SubmissionTrack>, JsfsError> {
        let mut stmt = self.conn.prepare(
            "SELECT journal_key, status, submitted_date, notes, updated_at
             FROM submission_tracker",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SubmissionTrack {
                journal_key: r.get(0)?,
                status: r.get(1)?,
                submitted_date: r.get(2)?,
                notes: r.get(3)?,
                updated_at: r.get(4)?,
            })
        })?;
        let mut map = HashMap::new();
        for item in rows.flatten() {
            map.insert(item.journal_key.clone(), item);
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(title: &str, incomplete: bool) -> JournalRecord {
        JournalRecord {
            title: title.to_string(),
            website_url: "https://example.ac.id/index.php/jurnal".to_string(),
            apc_status: "Gratis (Rp 0)".to_string(),
            publication_fee_idr: Some(0),
            is_incomplete: incomplete,
            cache_version: CACHE_VERSION.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_cache_roundtrip_and_incomplete_bypass() {
        let db = JournalCache::open_in_memory().unwrap();
        let complete = sample_record("Jurnal Lengkap", false);
        db.put(&complete.website_url, &complete).unwrap();
        assert_eq!(db.count_valid().unwrap(), 1);
        assert!(db.get(&complete.website_url).unwrap().is_some());

        let incomplete = sample_record("Jurnal Timeout", true);
        db.put("https://timeout.ac.id/index.php/j", &incomplete)
            .unwrap();
        // Incomplete records must be bypassed on read
        assert!(db
            .get("https://timeout.ac.id/index.php/j")
            .unwrap()
            .is_none());
        assert_eq!(db.count_valid().unwrap(), 1);
    }

    #[test]
    fn test_cache_ttl_and_version_purge() {
        let db = JournalCache::open_in_memory().unwrap();
        // Inject expired entry (20 days old) and legacy version entry
        let stale_ts = Utc::now().timestamp() - (20 * 86_400);
        let json = serde_json::to_string(&sample_record("Old", false)).unwrap();
        db.conn
            .execute(
                "INSERT INTO journal_cache (url, cache_version, fetched_at, is_incomplete, payload_json)
                 VALUES (?1, ?2, ?3, 0, ?4)",
                params!["https://expired.ac.id", CACHE_VERSION, stale_ts, json],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO journal_cache (url, cache_version, fetched_at, is_incomplete, payload_json)
                 VALUES (?1, ?2, ?3, 0, ?4)",
                params!["https://legacy.ac.id", "2.4.0", Utc::now().timestamp(), json],
            )
            .unwrap();

        assert!(db.get("https://expired.ac.id").unwrap().is_none());
        let purged = db.purge_stale_versions(CACHE_VERSION).unwrap();
        assert_eq!(purged, 1);
    }

    #[test]
    fn test_search_cached_journals_and_submission_tracker() {
        let db = JournalCache::open_in_memory().unwrap();
        let mut r1 = sample_record("Jurnal Rekayasa Perangkat Lunak", false);
        r1.website_url = "https://rpl.ac.id/index.php/jurnal".to_string();
        r1.focus_and_scope = "Software engineering, machine learning, web application".to_string();
        db.put(&r1.website_url, &r1).unwrap();

        let mut r2 = sample_record("Jurnal Agribisnis Pangan", false);
        r2.website_url = "https://agro.ac.id/index.php/j".to_string();
        r2.focus_and_scope = "Pertanian organik dan ketahanan pangan".to_string();
        db.put(&r2.website_url, &r2).unwrap();

        let hits = db.search_cached_journals("machine learning", 50).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Jurnal Rekayasa Perangkat Lunak");

        let key = SubmissionTrack::canonical_key(&r1.website_url, &r1.title);
        let track = SubmissionTrack {
            journal_key: key.clone(),
            status: "Under Review".to_string(),
            submitted_date: "2026-04-10".to_string(),
            notes: "Menunggu komentar reviewer 1".to_string(),
            updated_at: String::new(),
        };
        db.upsert_submission_track(&track).unwrap();
        let loaded = db.get_submission_track(&key).unwrap().unwrap();
        assert_eq!(loaded.status, "Under Review");
        assert_eq!(loaded.submitted_date, "2026-04-10");
        assert_eq!(db.load_all_submission_tracks().unwrap().len(), 1);
    }

    #[test]
    fn test_conditional_caching_and_touch() {
        let db = JournalCache::open_in_memory().unwrap();
        let rec = sample_record("Jurnal Sistem Cerdas", false);
        let url = "https://cerdas.ac.id/index.php/jsc";
        db.put_with_headers(
            url,
            &rec,
            Some("\"etag-12345\""),
            Some("Wed, 21 Oct 2026 07:28:00 GMT"),
        )
        .unwrap();

        let headers = db.get_conditional_headers(url).unwrap().unwrap();
        assert_eq!(headers.0.as_deref(), Some("\"etag-12345\""));
        assert_eq!(headers.1.as_deref(), Some("Wed, 21 Oct 2026 07:28:00 GMT"));

        assert!(db.touch_entry(url).unwrap());
        assert!(!db.touch_entry("https://unknown.ac.id").unwrap());
    }
}

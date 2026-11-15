use crate::config::{cache_db_path, CACHE_TTL_DAYS, CACHE_VERSION, SQLITE_CACHE_SIZE_KIB};
use crate::error::JsfsError;
use crate::models::JournalRecord;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

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
                payload_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_cache_ver_time
                ON journal_cache(cache_version, fetched_at);
            ",
        )?;
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
                    Ok(rec) => Ok(Some(rec)),
                    Err(_) => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    pub fn put(&self, url: &str, record: &JournalRecord) -> Result<(), JsfsError> {
        let key = Self::normalize_url(url);
        if key.is_empty() {
            return Ok(());
        }
        let now = Utc::now().timestamp();
        let json = serde_json::to_string(record).unwrap_or_default();
        self.conn.execute(
            "INSERT INTO journal_cache (url, cache_version, fetched_at, is_incomplete, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(url) DO UPDATE SET
               cache_version = excluded.cache_version,
               fetched_at = excluded.fetched_at,
               is_incomplete = excluded.is_incomplete,
               payload_json = excluded.payload_json",
            params![
                key,
                CACHE_VERSION,
                now,
                i32::from(record.is_incomplete),
                json
            ],
        )?;
        Ok(())
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
}

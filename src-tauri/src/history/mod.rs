use crate::dictionary::{CustomWord, Replacement};
use crate::polish::Correction;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptRow {
    pub id: i64,
    pub raw: String,
    pub polished: String,
    pub ts: i64,
    pub duration_ms: i64,
    pub provider_stt: String,
    pub provider_polish: Option<String>,
}

pub struct HistoryStore {
    conn: Mutex<Connection>,
}

impl HistoryStore {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_transcript(
        &self,
        raw: &str,
        polished: &str,
        duration_ms: i64,
        provider_stt: &str,
        provider_polish: Option<&str>,
    ) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO transcripts (raw, polished, ts, duration_ms, provider_stt, provider_polish)
             VALUES (?1, ?2, strftime('%s','now') * 1000, ?3, ?4, ?5)",
            params![raw, polished, duration_ms, provider_stt, provider_polish],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_transcript(&self, id: i64) -> anyhow::Result<Option<TranscriptRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, raw, polished, ts, duration_ms, provider_stt, provider_polish
             FROM transcripts WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![id], row_to_transcript).ok();
        Ok(row)
    }

    pub fn list_recent(&self, limit: u32) -> anyhow::Result<Vec<TranscriptRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, raw, polished, ts, duration_ms, provider_stt, provider_polish
             FROM transcripts ORDER BY ts DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], row_to_transcript)?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        self.conn.lock().execute("DELETE FROM transcripts", [])?;
        Ok(())
    }

    pub fn prune_to(&self, keep: u32) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM transcripts WHERE id NOT IN
             (SELECT id FROM transcripts ORDER BY ts DESC LIMIT ?1)",
            params![keep],
        )?;
        Ok(())
    }

    // -- custom words ----------------------------------------------------

    pub fn list_custom_words(&self) -> anyhow::Result<Vec<CustomWord>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, word, weight, created_at FROM custom_words ORDER BY weight DESC, id ASC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(CustomWord {
                    id: row.get(0)?,
                    word: row.get(1)?,
                    weight: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    pub fn add_custom_word(&self, word: &str, weight: i64) -> anyhow::Result<()> {
        self.conn.lock().execute(
            "INSERT OR REPLACE INTO custom_words (word, weight, created_at)
             VALUES (?1, ?2, strftime('%s','now') * 1000)",
            params![word, weight],
        )?;
        Ok(())
    }

    pub fn delete_custom_word(&self, id: i64) -> anyhow::Result<()> {
        self.conn
            .lock()
            .execute("DELETE FROM custom_words WHERE id = ?1", params![id])?;
        Ok(())
    }

    // -- replacements ----------------------------------------------------

    pub fn list_replacements(&self) -> anyhow::Result<Vec<Replacement>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, trigger, replacement, case_sensitive FROM replacements ORDER BY id ASC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Replacement {
                    id: row.get(0)?,
                    trigger: row.get(1)?,
                    replacement: row.get(2)?,
                    case_sensitive: row.get::<_, i64>(3)? != 0,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    pub fn upsert_replacement(
        &self,
        trigger: &str,
        replacement: &str,
        case_sensitive: bool,
    ) -> anyhow::Result<()> {
        let cs: i64 = if case_sensitive { 1 } else { 0 };
        self.conn.lock().execute(
            "INSERT INTO replacements (trigger, replacement, case_sensitive) VALUES (?1, ?2, ?3)
             ON CONFLICT(trigger) DO UPDATE SET replacement = excluded.replacement, case_sensitive = excluded.case_sensitive",
            params![trigger, replacement, cs],
        )?;
        Ok(())
    }

    pub fn delete_replacement(&self, id: i64) -> anyhow::Result<()> {
        self.conn
            .lock()
            .execute("DELETE FROM replacements WHERE id = ?1", params![id])?;
        Ok(())
    }

    // -- corrections (for style-learning few-shots) ----------------------

    pub fn add_correction(&self, raw: &str, final_text: &str) -> anyhow::Result<()> {
        self.conn.lock().execute(
            "INSERT INTO corrections (transcript_raw, transcript_final, ts)
             VALUES (?1, ?2, strftime('%s','now') * 1000)",
            params![raw, final_text],
        )?;
        Ok(())
    }

    pub fn recent_corrections(&self, limit: u32) -> anyhow::Result<Vec<Correction>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT transcript_raw, transcript_final FROM corrections ORDER BY ts DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |row| {
                Ok(Correction {
                    raw: row.get(0)?,
                    final_text: row.get(1)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }
}

fn row_to_transcript(row: &rusqlite::Row<'_>) -> rusqlite::Result<TranscriptRow> {
    Ok(TranscriptRow {
        id: row.get(0)?,
        raw: row.get(1)?,
        polished: row.get(2)?,
        ts: row.get(3)?,
        duration_ms: row.get(4)?,
        provider_stt: row.get(5)?,
        provider_polish: row.get(6).ok(),
    })
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS transcripts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    raw TEXT NOT NULL,
    polished TEXT NOT NULL,
    ts INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    provider_stt TEXT NOT NULL,
    provider_polish TEXT
);
CREATE INDEX IF NOT EXISTS idx_transcripts_ts ON transcripts(ts DESC);

CREATE TABLE IF NOT EXISTS custom_words (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL UNIQUE,
    weight INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS replacements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trigger TEXT NOT NULL UNIQUE,
    replacement TEXT NOT NULL,
    case_sensitive INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS corrections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    transcript_raw TEXT NOT NULL,
    transcript_final TEXT NOT NULL,
    ts INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_corrections_ts ON corrections(ts DESC);
"#;

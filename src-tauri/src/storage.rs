use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

/// 一条验证码记录。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeRecord {
    pub id: i64,
    pub source: String,
    pub sender: Option<String>,
    pub body: String,
    pub code: String,
    /// Unix 毫秒时间戳
    pub received_at: i64,
    pub used: bool,
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn row_to_record(row: &Row) -> rusqlite::Result<CodeRecord> {
    Ok(CodeRecord {
        id: row.get(0)?,
        source: row.get(1)?,
        sender: row.get(2)?,
        body: row.get(3)?,
        code: row.get(4)?,
        received_at: row.get(5)?,
        used: row.get::<_, i64>(6)? != 0,
    })
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS codes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                sender TEXT,
                body TEXT NOT NULL,
                code TEXT NOT NULL,
                received_at INTEGER NOT NULL,
                used INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_codes_received_at ON codes (received_at DESC);",
        )
        .map_err(|e| e.to_string())
    }

    /// 插入一条记录并返回完整的 CodeRecord。
    pub fn insert(
        &self,
        source: &str,
        sender: Option<&str>,
        body: &str,
        code: &str,
        received_at: i64,
    ) -> Result<CodeRecord, String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO codes (source, sender, body, code, received_at, used)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![source, sender, body, code, received_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(CodeRecord {
            id: conn.last_insert_rowid(),
            source: source.to_string(),
            sender: sender.map(|s| s.to_string()),
            body: body.to_string(),
            code: code.to_string(),
            received_at,
            used: false,
        })
    }

    /// 列出记录；query 非空时按 sender/body/code 模糊过滤。按时间倒序，最多 500 条。
    pub fn list(&self, query: Option<&str>) -> Result<Vec<CodeRecord>, String> {
        let conn = self.conn.lock().unwrap();
        let mut records = Vec::new();
        let keyword = query.map(|q| q.trim().to_string()).filter(|q| !q.is_empty());
        if let Some(keyword) = keyword {
            let pattern = format!("%{keyword}%");
            let mut stmt = conn
                .prepare(
                    "SELECT id, source, sender, body, code, received_at, used FROM codes
                     WHERE sender LIKE ?1 OR body LIKE ?1 OR code LIKE ?1
                     ORDER BY received_at DESC, id DESC LIMIT 500",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![pattern], row_to_record)
                .map_err(|e| e.to_string())?;
            for row in rows {
                records.push(row.map_err(|e| e.to_string())?);
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, source, sender, body, code, received_at, used FROM codes
                     ORDER BY received_at DESC, id DESC LIMIT 500",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], row_to_record).map_err(|e| e.to_string())?;
            for row in rows {
                records.push(row.map_err(|e| e.to_string())?);
            }
        }
        Ok(records)
    }

    pub fn get(&self, id: i64) -> Result<Option<CodeRecord>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, source, sender, body, code, received_at, used FROM codes
                 WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
        match rows.next().map_err(|e| e.to_string())? {
            Some(row) => Ok(Some(row_to_record(row).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    /// 最新一条记录（按接收时间倒序）。
    pub fn latest(&self) -> Result<Option<CodeRecord>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, source, sender, body, code, received_at, used FROM codes
                 ORDER BY received_at DESC, id DESC LIMIT 1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        match rows.next().map_err(|e| e.to_string())? {
            Some(row) => Ok(Some(row_to_record(row).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    pub fn clear(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM codes", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM codes WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn mark_used(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE codes SET used = 1 WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 清理超过保留期的记录；retention_days <= 0 表示永久保留，不清理。
    pub fn cleanup(&self, retention_days: i64) -> Result<(), String> {
        if retention_days <= 0 {
            return Ok(());
        }
        let cutoff = now_millis() - retention_days * 86_400_000;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM codes WHERE received_at < ?1",
            params![cutoff],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

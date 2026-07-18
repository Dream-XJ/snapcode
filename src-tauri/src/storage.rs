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

/// 去重表中的基线哨兵 UIDL：存在表示「首次轮询只建基线」已完成，之后的邮件才会被导入。
pub const EMAIL_BASELINE_UIDL: &str = "__baseline__";

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
            CREATE INDEX IF NOT EXISTS idx_codes_received_at ON codes (received_at DESC);
            -- 已处理邮件的 UIDL 去重表（POP3 轮询）；含一条 __baseline__ 哨兵行表示基线已建立
            CREATE TABLE IF NOT EXISTS email_seen (
                uidl TEXT PRIMARY KEY,
                seen_at INTEGER NOT NULL
            );",
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

    /* ---------- 邮件 UIDL 去重表 ---------- */

    /// 取出全部已见 UIDL（含基线哨兵）。
    pub fn email_seen_set(&self) -> Result<std::collections::HashSet<String>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT uidl FROM email_seen")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut set = std::collections::HashSet::new();
        for row in rows {
            set.insert(row.map_err(|e| e.to_string())?);
        }
        Ok(set)
    }

    /// 批量标记 UIDL 为已见（重复忽略）。
    pub fn email_seen_mark(&self, uidls: &[String]) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("INSERT OR IGNORE INTO email_seen (uidl, seen_at) VALUES (?1, ?2)")
            .map_err(|e| e.to_string())?;
        let now = now_millis();
        for uidl in uidls {
            stmt.execute(params![uidl, now]).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 清空去重表：邮箱账户（host/username）变更后 UIDL 命名空间已不同，需重建基线。
    pub fn email_seen_clear(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM email_seen", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 清理过旧的 UIDL 记录，防止去重表无限增长；哨兵行保留。
    pub fn email_seen_cleanup(&self, keep_days: i64) -> Result<(), String> {
        let cutoff = now_millis() - keep_days.max(1) * 86_400_000;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM email_seen WHERE seen_at < ?1 AND uidl != ?2",
            params![cutoff, EMAIL_BASELINE_UIDL],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_db() -> Db {
        Db::open(Path::new(":memory:")).unwrap()
    }

    /// UIDL 标记→查询→重复标记（幂等）→清空。
    #[test]
    fn email_seen_mark_and_clear() {
        let db = mem_db();
        assert!(db.email_seen_set().unwrap().is_empty());

        db.email_seen_mark(&["a".to_string(), "b".to_string()]).unwrap();
        db.email_seen_mark(&["a".to_string()]).unwrap(); // 重复标记不报错
        let set = db.email_seen_set().unwrap();
        assert_eq!(set.len(), 2);
        assert!(set.contains("a") && set.contains("b"));

        db.email_seen_clear().unwrap();
        assert!(db.email_seen_set().unwrap().is_empty());
    }

    /// 清理保留哨兵行与近期记录，删除过期记录。
    #[test]
    fn email_seen_cleanup_keeps_baseline() {
        let db = mem_db();
        db.email_seen_mark(&[EMAIL_BASELINE_UIDL.to_string(), "old".to_string()])
            .unwrap();
        // 把 "old" 的 seen_at 改到 40 天前（哨兵保持当前时间）
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE email_seen SET seen_at = ?1 WHERE uidl = 'old'",
                params![now_millis() - 40 * 86_400_000],
            )
            .unwrap();
        }
        db.email_seen_cleanup(30).unwrap();
        let set = db.email_seen_set().unwrap();
        assert!(set.contains(EMAIL_BASELINE_UIDL));
        assert!(!set.contains("old"));
    }
}

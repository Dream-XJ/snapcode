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
        // email_seen 旧版结构为 (uidl PRIMARY KEY, seen_at)，无账户维度。
        // 邮箱功能尚未随正式版发布，去重缓存可直接丢弃：检测到旧表结构时
        // DROP 重建，各账户下次轮询重新建立基线即可。
        let legacy_email_seen = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(email_seen)")
                .map_err(|e| e.to_string())?;
            let cols = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<String>, _>>()
                .map_err(|e| e.to_string())?;
            !cols.is_empty() && !cols.iter().any(|c| c == "account_id")
        };
        if legacy_email_seen {
            conn.execute("DROP TABLE email_seen", [])
                .map_err(|e| e.to_string())?;
        }
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
            -- 已处理邮件的 UIDL 去重表（按账户隔离）；每账户一条 __baseline__ 哨兵行表示该账户基线已建立
            CREATE TABLE IF NOT EXISTS email_seen (
                account_id TEXT NOT NULL,
                uidl TEXT NOT NULL,
                seen_at INTEGER NOT NULL,
                PRIMARY KEY (account_id, uidl)
            );
            -- 各账户的 IMAP 同步状态（UIDVALIDITY 与已处理的最大 UID）
            CREATE TABLE IF NOT EXISTS imap_state (
                account_id TEXT PRIMARY KEY,
                uidvalidity INTEGER NOT NULL,
                max_uid INTEGER NOT NULL
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

    /* ---------- 邮件 UIDL 去重表（按账户隔离） ---------- */

    /// 取出指定账户的全部已见 UIDL（含基线哨兵）。
    pub fn email_seen_set(&self, account: &str) -> Result<std::collections::HashSet<String>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT uidl FROM email_seen WHERE account_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![account], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut set = std::collections::HashSet::new();
        for row in rows {
            set.insert(row.map_err(|e| e.to_string())?);
        }
        Ok(set)
    }

    /// 批量标记指定账户的 UIDL 为已见（重复忽略）。
    pub fn email_seen_mark(&self, account: &str, uidls: &[String]) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("INSERT OR IGNORE INTO email_seen (account_id, uidl, seen_at) VALUES (?1, ?2, ?3)")
            .map_err(|e| e.to_string())?;
        let now = now_millis();
        for uidl in uidls {
            stmt.execute(params![account, uidl, now])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 清空指定账户的去重记录：账户身份（host/username）变更或账户被删除后
    /// UIDL 命名空间已不同，需重建基线。
    pub fn email_seen_clear(&self, account: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM email_seen WHERE account_id = ?1",
            params![account],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 清理过旧的 UIDL 记录（跨全部账户），防止去重表无限增长；各账户的哨兵行保留。
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

    /* ---------- IMAP 同步状态（按账户） ---------- */

    /// 读取指定账户的 IMAP 同步状态（UIDVALIDITY, 已处理的最大 UID）；从未同步返回 None。
    pub fn imap_state_get(&self, account: &str) -> Result<Option<(u32, u32)>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT uidvalidity, max_uid FROM imap_state WHERE account_id = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![account]).map_err(|e| e.to_string())?;
        match rows.next().map_err(|e| e.to_string())? {
            Some(row) => Ok(Some((
                row.get::<_, u32>(0).map_err(|e| e.to_string())?,
                row.get::<_, u32>(1).map_err(|e| e.to_string())?,
            ))),
            None => Ok(None),
        }
    }

    /// 写入/覆盖指定账户的 IMAP 同步状态。
    pub fn imap_state_set(&self, account: &str, uidvalidity: u32, max_uid: u32) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO imap_state (account_id, uidvalidity, max_uid) VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id) DO UPDATE SET uidvalidity = ?2, max_uid = ?3",
            params![account, uidvalidity, max_uid],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 清除指定账户的 IMAP 同步状态（账户删除或身份变更时随去重记录一并重置）。
    pub fn imap_state_clear(&self, account: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM imap_state WHERE account_id = ?1",
            params![account],
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

    /// 同一 UIDL 在两个账户下互不影响；clear 只清指定账户；重复标记幂等。
    #[test]
    fn email_seen_isolated_per_account() {
        let db = mem_db();
        assert!(db.email_seen_set("acc1").unwrap().is_empty());

        db.email_seen_mark("acc1", &["a".to_string(), "b".to_string()])
            .unwrap();
        db.email_seen_mark("acc1", &["a".to_string()]).unwrap(); // 重复标记不报错
        db.email_seen_mark("acc2", &["a".to_string()]).unwrap();

        let set1 = db.email_seen_set("acc1").unwrap();
        let set2 = db.email_seen_set("acc2").unwrap();
        assert_eq!(set1.len(), 2);
        assert!(set1.contains("a") && set1.contains("b"));
        assert_eq!(set2.len(), 1);
        assert!(set2.contains("a"));

        db.email_seen_clear("acc1").unwrap();
        assert!(db.email_seen_set("acc1").unwrap().is_empty());
        assert_eq!(db.email_seen_set("acc2").unwrap().len(), 1);
    }

    /// 基线哨兵按账户独立。
    #[test]
    fn baseline_sentinel_is_per_account() {
        let db = mem_db();
        db.email_seen_mark("acc1", &[EMAIL_BASELINE_UIDL.to_string()])
            .unwrap();
        assert!(db
            .email_seen_set("acc1")
            .unwrap()
            .contains(EMAIL_BASELINE_UIDL));
        assert!(!db
            .email_seen_set("acc2")
            .unwrap()
            .contains(EMAIL_BASELINE_UIDL));
    }

    /// 清理跨账户删除过期记录，但豁免各账户的哨兵行。
    #[test]
    fn email_seen_cleanup_keeps_baseline_per_account() {
        let db = mem_db();
        db.email_seen_mark("acc1", &[EMAIL_BASELINE_UIDL.to_string(), "old".to_string()])
            .unwrap();
        db.email_seen_mark(
            "acc2",
            &[
                EMAIL_BASELINE_UIDL.to_string(),
                "old".to_string(),
                "new".to_string(),
            ],
        )
        .unwrap();
        // 把两个账户的 "old" 的 seen_at 都改到 40 天前（哨兵与 "new" 保持当前时间）
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE email_seen SET seen_at = ?1 WHERE uidl = 'old'",
                params![now_millis() - 40 * 86_400_000],
            )
            .unwrap();
        }
        db.email_seen_cleanup(30).unwrap();
        let set1 = db.email_seen_set("acc1").unwrap();
        let set2 = db.email_seen_set("acc2").unwrap();
        assert!(set1.contains(EMAIL_BASELINE_UIDL));
        assert!(set2.contains(EMAIL_BASELINE_UIDL));
        assert!(!set1.contains("old"));
        assert!(!set2.contains("old"));
        assert!(set2.contains("new"));
    }

    /// 旧版两列 email_seen 表在打开时被检测并 DROP 重建（旧缓存丢弃，账户重建基线）。
    #[test]
    fn legacy_email_seen_table_is_recreated() {
        let dir = std::env::temp_dir().join(format!("snapcode-db-migrate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let _ = std::fs::remove_file(&path);

        // 手工建旧版两列表并写入一条记录
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE email_seen (uidl TEXT PRIMARY KEY, seen_at INTEGER NOT NULL);
                 INSERT INTO email_seen (uidl, seen_at) VALUES ('legacy-uidl', 1);",
            )
            .unwrap();
        }

        // Db::open 迁移后：旧数据已丢弃，新表带账户维度且可正常读写
        let db = Db::open(&path).unwrap();
        assert!(db.email_seen_set("acc1").unwrap().is_empty());
        db.email_seen_mark("acc1", &["x".to_string()]).unwrap();
        assert!(db.email_seen_set("acc1").unwrap().contains("x"));

        drop(db);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    /// IMAP 同步状态的读写、覆盖更新、账户隔离与清除。
    #[test]
    fn imap_state_roundtrip() {
        let db = mem_db();
        assert_eq!(db.imap_state_get("acc1").unwrap(), None);

        db.imap_state_set("acc1", 42, 100).unwrap();
        assert_eq!(db.imap_state_get("acc1").unwrap(), Some((42, 100)));

        // 覆盖更新（max_uid 推进）
        db.imap_state_set("acc1", 42, 150).unwrap();
        assert_eq!(db.imap_state_get("acc1").unwrap(), Some((42, 150)));

        // 账户间隔离
        assert_eq!(db.imap_state_get("acc2").unwrap(), None);

        db.imap_state_clear("acc1").unwrap();
        assert_eq!(db.imap_state_get("acc1").unwrap(), None);
    }
}

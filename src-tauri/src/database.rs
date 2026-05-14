/// database.rs — SQLite persistent storage for Map Scraper
/// 
/// Database file location:
///   Dev:     ./data/leads.db (relative to working dir)
///   Packaged: C:\Users\<user>\AppData\Local\com.detau.map-scraper\leads.db
///
/// Key design decisions:
///   - `phone` column has UNIQUE index → INSERT OR IGNORE handles cross-session dedup automatically
///   - `bundled` feature in rusqlite compiles SQLite into the binary → no DLL needed in .exe

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ============================================================
// Types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbLead {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub address: String,
    pub region: String,
    pub keyword: String,
    pub website: String,
    pub email: String,
    pub scraped_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub total_leads: i64,
    pub unique_phones: i64,
    pub regions: Vec<RegionCount>,
    pub keywords: Vec<KeywordCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionCount {
    pub region: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordCount {
    pub keyword: String,
    pub count: i64,
}

// ============================================================
// Database
// ============================================================

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the SQLite database at the given path.
    /// Tables are created automatically on first run.
    pub fn open(db_path: &Path) -> SqlResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(db_path)?;

        // Performance tuning (safe for desktop use)
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
        ")?;

        let db = Database { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> SqlResult<()> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS leads (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT    NOT NULL,
                phone       TEXT    NOT NULL DEFAULT '',
                address     TEXT    NOT NULL DEFAULT '',
                region      TEXT    NOT NULL DEFAULT '',
                keyword     TEXT    NOT NULL DEFAULT '',
                website     TEXT    NOT NULL DEFAULT '',
                email       TEXT    NOT NULL DEFAULT '',
                scraped_at  TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
            );

            -- Attempt to add columns if upgrading from old version
            ALTER TABLE leads ADD COLUMN website TEXT DEFAULT '';
            ALTER TABLE leads ADD COLUMN email TEXT DEFAULT '';

            -- Unique index on normalized phone (non-empty only)
            CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_phone
                ON leads(phone)
                WHERE phone != '';

            -- Settings key-value table
            CREATE TABLE IF NOT EXISTS settings (
                key     TEXT PRIMARY KEY,
                value   TEXT NOT NULL DEFAULT ''
            );

            -- Zalo activity log
            CREATE TABLE IF NOT EXISTS zalo_activities (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                lead_id     INTEGER REFERENCES leads(id),
                action      TEXT NOT NULL,
                result      TEXT NOT NULL DEFAULT 'SUCCESS',
                note        TEXT NOT NULL DEFAULT '',
                created_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );
        ")?;

        // Migrations: add Zalo CRM columns to leads (ignore errors if already exist)
        let migrations = [
            "ALTER TABLE leads ADD COLUMN status TEXT NOT NULL DEFAULT 'NEW'",
            "ALTER TABLE leads ADD COLUMN zalo_found INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE leads ADD COLUMN friend_sent_at TEXT",
            "ALTER TABLE leads ADD COLUMN friend_accepted_at TEXT",
            "ALTER TABLE leads ADD COLUMN first_msg_sent_at TEXT",
            "ALTER TABLE leads ADD COLUMN last_reply_at TEXT",
            "ALTER TABLE leads ADD COLUMN note TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE leads ADD COLUMN converted_at TEXT",
        ];
        for sql in &migrations {
            let _ = self.conn.execute(sql, []); // ignore "duplicate column" errors
        }

        // Insert default settings if not exist
        let defaults = [
            ("zalo_msg_template", "Xin chào [Tên]! Mình là đại lý giày dép, muốn giới thiệu sản phẩm đến bạn. Bạn có quan tâm không ạ?"),
            ("zalo_delay_min", "60"),
            ("zalo_delay_max", "120"),
            ("zalo_daily_limit", "20"),
        ];
        for (key, value) in &defaults {
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }

        Ok(())
    }

    // ── Deduplication Check ──────────────────────────────────

    /// Check if a normalized phone number already exists in the database.
    /// This is the global cross-session dedup check.
    pub fn phone_exists(&self, normalized_phone: &str) -> bool {
        if normalized_phone.is_empty() {
            return false;
        }
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM leads WHERE phone = ?1",
                params![normalized_phone],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0
    }

    // ── Write ─────────────────────────────────────────────────

    /// Insert a lead into the database.
    /// Returns true if inserted, false if skipped (duplicate phone).
    pub fn insert_lead(
        &self,
        name: &str,
        phone: &str,
        address: &str,
        region: &str,
        keyword: &str,
        website: &str,
        email: &str,
    ) -> SqlResult<bool> {
        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO leads (name, phone, address, region, keyword, website, email)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![name, phone, address, region, keyword, website, email],
        )?;
        Ok(rows > 0)
    }

    // ── Read ─────────────────────────────────────────────────

    /// Get all leads, newest first.
    pub fn get_all_leads(&self) -> SqlResult<Vec<DbLead>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, phone, address, region, keyword, website, email, scraped_at
             FROM leads
             ORDER BY id DESC",
        )?;

        let leads = stmt
            .query_map([], |row| {
                Ok(DbLead {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    phone: row.get(2)?,
                    address: row.get(3)?,
                    region: row.get(4)?,
                    keyword: row.get(5)?,
                    website: row.get(6).unwrap_or_default(),
                    email: row.get(7).unwrap_or_default(),
                    scraped_at: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(leads)
    }

    /// Get leads filtered by region.
    pub fn get_leads_by_region(&self, region: &str) -> SqlResult<Vec<DbLead>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, phone, address, region, keyword, website, email, scraped_at
             FROM leads
             WHERE region LIKE ?1
             ORDER BY id DESC",
        )?;

        let pattern = format!("%{}%", region);
        let leads = stmt
            .query_map(params![pattern], |row| {
                Ok(DbLead {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    phone: row.get(2)?,
                    address: row.get(3)?,
                    region: row.get(4)?,
                    keyword: row.get(5)?,
                    website: row.get(6).unwrap_or_default(),
                    email: row.get(7).unwrap_or_default(),
                    scraped_at: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(leads)
    }

    // ── Statistics ────────────────────────────────────────────

    pub fn get_stats(&self) -> SqlResult<DbStats> {
        let total_leads: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM leads", [], |r| r.get(0))
            .unwrap_or(0);

        let unique_phones: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT phone) FROM leads WHERE phone != ''",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Top regions
        let mut stmt = self.conn.prepare(
            "SELECT region, COUNT(*) as cnt FROM leads
             WHERE region != ''
             GROUP BY region ORDER BY cnt DESC LIMIT 10",
        )?;
        let regions = stmt
            .query_map([], |row| {
                Ok(RegionCount {
                    region: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Top keywords
        let mut stmt2 = self.conn.prepare(
            "SELECT keyword, COUNT(*) as cnt FROM leads
             WHERE keyword != ''
             GROUP BY keyword ORDER BY cnt DESC LIMIT 10",
        )?;
        let keywords = stmt2
            .query_map([], |row| {
                Ok(KeywordCount {
                    keyword: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(DbStats {
            total_leads,
            unique_phones,
            regions,
            keywords,
        })
    }

    // ── Maintenance ───────────────────────────────────────────

    pub fn clear_all(&self) -> SqlResult<usize> {
        self.conn.execute("DELETE FROM leads", [])
    }

    pub fn clear_by_region(&self, region: &str) -> SqlResult<usize> {
        self.conn.execute(
            "DELETE FROM leads WHERE region LIKE ?1",
            params![format!("%{}%", region)],
        )
    }

    // ── CRM Methods ───────────────────────────────────────────

    /// Update a lead's status.
    pub fn update_lead_status(&self, id: i64, status: &str) -> SqlResult<()> {
        let now = "datetime('now', 'localtime')";
        let extra_col = match status {
            "CUSTOMER"        => format!(", converted_at = {}", now),
            "FRIEND_ACCEPTED" => format!(", friend_accepted_at = {}", now),
            "REPLIED"         => format!(", last_reply_at = {}", now),
            _ => String::new(),
        };
        self.conn.execute(
            &format!("UPDATE leads SET status = ?1{} WHERE id = ?2", extra_col),
            params![status, id],
        )?;
        Ok(())
    }

    /// Update a lead's note.
    pub fn update_lead_note(&self, id: i64, note: &str) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE leads SET note = ?1 WHERE id = ?2",
            params![note, id],
        )?;
        Ok(())
    }

    /// Mark a lead as friend request sent.
    pub fn mark_friend_sent(&self, id: i64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE leads SET status = 'SENT_FRIEND', friend_sent_at = datetime('now','localtime') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Mark a lead as message sent.
    pub fn mark_msg_sent(&self, id: i64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE leads SET status = 'SENT_MSG', first_msg_sent_at = datetime('now','localtime') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Mark whether Zalo account was found for a lead's phone.
    pub fn set_zalo_found(&self, id: i64, found: bool) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE leads SET zalo_found = ?1 WHERE id = ?2",
            params![if found { 1 } else { 0 }, id],
        )?;
        Ok(())
    }

    /// Get all leads with CRM status info.
    pub fn get_crm_leads(&self, status_filter: Option<&str>) -> SqlResult<Vec<CrmLead>> {
        let sql = if let Some(s) = status_filter {
            format!(
                "SELECT id, name, phone, address, region, keyword, status, zalo_found,
                        friend_sent_at, friend_accepted_at, first_msg_sent_at, last_reply_at,
                        note, scraped_at, converted_at
                 FROM leads WHERE status = '{}' ORDER BY id DESC", s
            )
        } else {
            "SELECT id, name, phone, address, region, keyword, status, zalo_found,
                    friend_sent_at, friend_accepted_at, first_msg_sent_at, last_reply_at,
                    note, scraped_at, converted_at
             FROM leads ORDER BY id DESC".to_string()
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let leads = stmt
            .query_map([], |row| {
                Ok(CrmLead {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    phone: row.get(2)?,
                    address: row.get(3)?,
                    region: row.get(4)?,
                    keyword: row.get(5)?,
                    status: row.get(6)?,
                    zalo_found: row.get::<_, i64>(7).unwrap_or(0) == 1,
                    friend_sent_at: row.get(8)?,
                    friend_accepted_at: row.get(9)?,
                    first_msg_sent_at: row.get(10)?,
                    last_reply_at: row.get(11)?,
                    note: row.get(12)?,
                    scraped_at: row.get(13)?,
                    converted_at: row.get(14)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(leads)
    }

    /// Count how many friend requests sent today (for rate limiting).
    pub fn count_today_sent(&self) -> i64 {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM leads WHERE date(friend_sent_at) = date('now','localtime')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0)
    }

    /// Get CRM funnel stats.
    pub fn get_crm_stats(&self) -> SqlResult<CrmStats> {
        let count = |status: &str| -> i64 {
            self.conn
                .query_row(
                    "SELECT COUNT(*) FROM leads WHERE status = ?1",
                    params![status],
                    |r| r.get(0),
                )
                .unwrap_or(0)
        };
        Ok(CrmStats {
            new: count("NEW"),
            sent_friend: count("SENT_FRIEND"),
            sent_msg: count("SENT_MSG"),
            friend_accepted: count("FRIEND_ACCEPTED"),
            replied: count("REPLIED"),
            customer: count("CUSTOMER"),
            rejected: count("REJECTED"),
            today_sent: self.count_today_sent(),
        })
    }

    // ── Settings ─────────────────────────────────────────────

    pub fn get_setting(&self, key: &str) -> String {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .unwrap_or_default()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        if let Ok(mut stmt) = self.conn.prepare("SELECT key, value FROM settings") {
            if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))) {
                for row in rows.flatten() {
                    map.insert(row.0, row.1);
                }
            }
        }
        map
    }

    // ── Activity Log ─────────────────────────────────────────

    pub fn log_activity(&self, lead_id: i64, action: &str, result: &str, note: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO zalo_activities (lead_id, action, result, note) VALUES (?1, ?2, ?3, ?4)",
            params![lead_id, action, result, note],
        )?;
        Ok(())
    }
}

// ============================================================
// CRM Types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrmLead {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub address: String,
    pub region: String,
    pub keyword: String,
    pub status: String,
    pub zalo_found: bool,
    pub friend_sent_at: Option<String>,
    pub friend_accepted_at: Option<String>,
    pub first_msg_sent_at: Option<String>,
    pub last_reply_at: Option<String>,
    pub note: String,
    pub scraped_at: String,
    pub converted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrmStats {
    pub new: i64,
    pub sent_friend: i64,
    pub sent_msg: i64,
    pub friend_accepted: i64,
    pub replied: i64,
    pub customer: i64,
    pub rejected: i64,
    pub today_sent: i64,
}


use anyhow::Result;
use rusqlite::Connection;
use crate::models::hosts::{HostsContentObject, HostsListObject, TrashcanItem, HostsHistoryObject};
use crate::utils::platform;

pub struct SwhDb {
    conn: Connection,
}

impl SwhDb {
    /// 从已有连接创建（用于测试）
    pub fn from_connection(conn: Connection) -> Self {
        let db = Self { conn };
        db.init_tables().ok();
        db
    }

    /// 打开或创建 hostz.db
    pub fn open() -> Result<Self> {
        let data_dir = platform::data_dir();
        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("hostz.db");
        let conn = Connection::open(&db_path)?;

        // WAL 模式提升并发性能
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS hosts_content (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS list_tree (
                id TEXT PRIMARY KEY,
                parent_id TEXT,
                order_idx INTEGER NOT NULL DEFAULT 0,
                data TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS trashcan (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                add_time_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS history (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                add_time_ms INTEGER NOT NULL,
                label TEXT
            );

            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    // ---- hosts_content ----

    pub fn get_content(&self, id: &str) -> Result<Option<HostsContentObject>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content FROM hosts_content WHERE id = ?1"
        )?;
        let result = stmt.query_row([id], |row| {
            Ok(HostsContentObject {
                id: row.get(0)?,
                content: row.get(1)?,
            })
        });
        match result {
            Ok(obj) => Ok(Some(obj)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_content(&self, id: &str, content: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO hosts_content (id, content) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET content = excluded.content",
            rusqlite::params![id, content],
        )?;
        Ok(())
    }

    pub fn delete_content(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM hosts_content WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    // ---- list_tree ----

    pub fn get_list(&self) -> Result<Vec<HostsListObject>> {
        let mut stmt = self.conn.prepare(
            "SELECT data FROM list_tree ORDER BY order_idx"
        )?;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut list = Vec::new();
        for row in rows {
            let obj: HostsListObject = serde_json::from_str(&row?)?;
            list.push(obj);
        }
        Ok(list)
    }

    pub fn set_list(&self, list: &[HostsListObject]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM list_tree", [])?;

        // 先写入顶层节点
        {
            let mut stmt = tx.prepare(
                "INSERT INTO list_tree (id, parent_id, order_idx, data) VALUES (?1, NULL, ?2, ?3)"
            )?;
            for (i, item) in list.iter().enumerate() {
                let json = serde_json::to_string(item)?;
                stmt.execute(rusqlite::params![item.id, i as i32, json])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    // ---- trashcan ----

    pub fn get_trashcan(&self) -> Result<Vec<TrashcanItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT data FROM trashcan ORDER BY add_time_ms DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut list = Vec::new();
        for row in rows {
            let item: TrashcanItem = serde_json::from_str(&row?)?;
            list.push(item);
        }
        Ok(list)
    }

    pub fn add_to_trashcan(&self, item: &TrashcanItem) -> Result<()> {
        let json = serde_json::to_string(item)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO trashcan (id, data, add_time_ms) VALUES (?1, ?2, ?3)",
            rusqlite::params![item.data.id, json, item.add_time_ms],
        )?;
        Ok(())
    }

    pub fn remove_from_trashcan(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM trashcan WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn clear_trashcan(&self) -> Result<()> {
        self.conn.execute("DELETE FROM trashcan", [])?;
        Ok(())
    }

    // ---- history ----

    pub fn add_history(&self, entry: &HostsHistoryObject) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO history (id, content, add_time_ms, label) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![entry.id, entry.content, entry.add_time_ms, entry.label],
        )?;
        Ok(())
    }

    pub fn get_history(&self, limit: i32) -> Result<Vec<HostsHistoryObject>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, add_time_ms, label FROM history ORDER BY add_time_ms DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map([limit], |row| {
            Ok(HostsHistoryObject {
                id: row.get(0)?,
                content: row.get(1)?,
                add_time_ms: row.get(2)?,
                label: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn delete_history(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM history WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn trim_history(&self, limit: i32) -> Result<()> {
        self.conn.execute(
            "DELETE FROM history WHERE id NOT IN (
                SELECT id FROM history ORDER BY add_time_ms DESC LIMIT ?1
            )",
            [limit],
        )?;
        Ok(())
    }

    // ---- meta ----

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
        let result = stmt.query_row([key], |row| row.get(0));
        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    // ---- 导入/导出 ----

    pub fn to_json(&self) -> Result<serde_json::Value> {
        let list = self.get_list()?;
        let trashcan = self.get_trashcan()?;
        let history = self.get_history(i32::MAX)?;

        Ok(serde_json::json!({
            "list": list,
            "trashcan": trashcan,
            "history": history,
        }))
    }

    pub fn load_json(&self, data: &serde_json::Value) -> Result<()> {
        // 先解析所有数据，验证无误后再写入
        let list: Vec<HostsListObject> = data.get("list")
            .and_then(|v| v.as_array())
            .map(|arr| serde_json::from_value(serde_json::Value::Array(arr.clone())))
            .transpose()?
            .unwrap_or_default();

        let trashcan: Vec<TrashcanItem> = data.get("trashcan")
            .and_then(|v| v.as_array())
            .map(|arr| serde_json::from_value(serde_json::Value::Array(arr.clone())))
            .transpose()?
            .unwrap_or_default();

        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM hosts_content", [])?;
        tx.execute("DELETE FROM list_tree", [])?;
        tx.execute("DELETE FROM trashcan", [])?;
        tx.execute("DELETE FROM history", [])?;

        // 恢复列表
        {
            let mut stmt = tx.prepare(
                "INSERT INTO list_tree (id, parent_id, order_idx, data) VALUES (?1, NULL, ?2, ?3)"
            )?;
            for (i, item) in list.iter().enumerate() {
                let json = serde_json::to_string(item)?;
                stmt.execute(rusqlite::params![item.id, i as i32, json])?;
            }
        }

        // 恢复回收站
        for item in &trashcan {
            let json = serde_json::to_string(item)?;
            tx.execute(
                "INSERT INTO trashcan (id, data, add_time_ms) VALUES (?1, ?2, ?3)",
                rusqlite::params![item.data.id, json, item.add_time_ms],
            )?;
        }

        tx.commit()?;
        Ok(())
    }
}

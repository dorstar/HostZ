use anyhow::Result;
use rusqlite::Connection;

use crate::models::config::AppConfig;
use crate::utils::platform;

pub struct CfgDb {
    conn: Connection,
}

impl CfgDb {
    /// 从已有连接创建（用于测试）
    pub fn from_connection(conn: Connection) -> Self {
        let db = Self { conn };
        db.init_tables().ok();
        db
    }

    /// 打开或创建 cfg.db
    pub fn open() -> Result<Self> {
        let config_dir = platform::config_dir();
        std::fs::create_dir_all(&config_dir)?;

        let db_path = config_dir.join("cfg.db");
        let conn = Connection::open(&db_path)?;

        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    /// 获取单个配置值（原始字符串）
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM config WHERE key = ?1")?;
        let result = stmt.query_row([key], |row| row.get(0));
        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 设置单个配置值
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    /// 设置单个配置值（JSON 序列化）
    pub fn set_json<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let json = serde_json::to_string(value)?;
        self.set(key, &json)
    }

    /// 获取所有配置（返回 key-value 对）
    pub fn all(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM config")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut configs = Vec::new();
        for row in rows {
            configs.push(row?);
        }
        Ok(configs)
    }

    /// 批量更新配置
    pub fn update_batch(&self, kv_pairs: &[(&str, &str)]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)"
            )?;
            for (key, value) in kv_pairs {
                stmt.execute(rusqlite::params![key, value])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// 删除配置项
    pub fn delete(&self, key: &str) -> Result<()> {
        self.conn.execute("DELETE FROM config WHERE key = ?1", [key])?;
        Ok(())
    }

    /// 加载完整配置（从 DB 读取 + 合并默认值）
    ///
    /// 先构造默认 AppConfig，然后遍历 DB 中的 key-value 对，
    /// 将每个 value 解析为 JSON 后覆盖默认值。
    pub fn load_config(&self) -> Result<AppConfig> {
        let defaults = AppConfig::default();
        let pairs = self.all()?;

        if pairs.is_empty() {
            return Ok(defaults);
        }

        let mut map = serde_json::Map::new();
        for (key, value) in &pairs {
            let json_val = serde_json::from_str(value)
                .unwrap_or_else(|_| serde_json::Value::String(value.clone()));
            map.insert(key.clone(), json_val);
        }

        let stored: AppConfig = serde_json::from_value(serde_json::Value::Object(map))
            .unwrap_or(defaults.clone());

        Ok(stored)
    }

    /// 应用部分配置更新
    ///
    /// 1. 将 JSON 中的每个 key 序列化后写入 DB
    /// 2. 返回变更前后的 key 列表和完整配置
    pub fn apply_config_update(
        &self,
        partial: &serde_json::Value,
    ) -> Result<(AppConfig, AppConfig, Vec<String>)> {
        let old_config = self.load_config()?;

        let changed_keys: Vec<String> = partial
            .as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        // 写入每个变更项（仅允许已知配置键）
        const ALLOWED_KEYS: &[&str] = &[
            "theme", "locale", "leftPanelWidth", "writeMode", "choiceMode",
            "historyLimit", "hideAtLaunch", "showTitleOnTray", "removeDuplicateRecords",
            "autoDownloadUpdate", "hideDockIcon", "trayMiniWindow",
            "multiChoseFolderSwitchAll", "cmdAfterHostsApply",
        ];
        if let Some(obj) = partial.as_object() {
            for key in obj.keys() {
                if !ALLOWED_KEYS.contains(&key.as_str()) {
                    return Err(anyhow::anyhow!("未知配置项: {}", key));
                }
            }
            let tx = self.conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)"
                )?;
                for (key, value) in obj {
                    let json_str = serde_json::to_string(value)?;
                    stmt.execute(rusqlite::params![key, json_str])?;
                }
            }
            tx.commit()?;
        }

        let new_config = self.load_config()?;
        Ok((old_config, new_config, changed_keys))
    }

    /// 保存完整配置到 DB
    pub fn save_config(&self, config: &AppConfig) -> Result<()> {
        let json = serde_json::to_value(config)?;
        if let Some(obj) = json.as_object() {
            let tx = self.conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)"
                )?;
                for (key, value) in obj {
                    let json_str = serde_json::to_string(value)?;
                    stmt.execute(rusqlite::params![key, json_str])?;
                }
            }
            tx.commit()?;
        }
        Ok(())
    }
}

use crate::model::History;
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

pub(crate) struct Store(Mutex<Connection>);

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS history (id TEXT PRIMARY KEY, time INTEGER NOT NULL, data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); PRAGMA user_version=2;")?;
        Ok(Self(Mutex::new(connection)))
    }
    pub fn save(&self, history: &History) -> Result<()> {
        let mut connection = self.0.lock().unwrap();
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT OR REPLACE INTO history VALUES (?1, ?2, ?3)",
            params![history.id, history.time, serde_json::to_string(history)?],
        )?;
        transaction.execute("DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY time DESC LIMIT 100)", [])?;
        transaction.commit()?;
        Ok(())
    }
    pub fn list(&self) -> Result<Vec<History>> {
        let connection = self.0.lock().unwrap();
        let mut statement =
            connection.prepare("SELECT data FROM history ORDER BY time DESC LIMIT 100")?;
        let values = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        values
            .into_iter()
            .map(|value| Ok(serde_json::from_str(&value)?))
            .collect()
    }
    pub fn clear(&self) -> Result<()> {
        self.0.lock().unwrap().execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn receive_dir(&self) -> Result<Option<PathBuf>> {
        let value: Option<String> = self
            .0
            .lock()
            .unwrap()
            .query_row(
                "SELECT value FROM settings WHERE key = 'receive_dir'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        value
            .map(|value| Ok(serde_json::from_str(&value)?))
            .transpose()
    }

    pub fn set_receive_dir(&self, path: Option<&Path>) -> Result<()> {
        let connection = self.0.lock().unwrap();
        if let Some(path) = path {
            connection.execute(
                "INSERT OR REPLACE INTO settings VALUES ('receive_dir', ?1)",
                params![serde_json::to_string(path)?],
            )?;
        } else {
            connection.execute("DELETE FROM settings WHERE key = 'receive_dir'", [])?;
        }
        Ok(())
    }
}

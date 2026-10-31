use crate::{model::Project, network::Tick};
use directories::ProjectDirs;
use rusqlite::{Connection, params};
use std::{
    error::Error,
    fs,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DATABASE_FILE: &str = "polygram.sqlite3";

pub struct LocalStore {
    connection: Connection,
}

#[derive(Debug, PartialEq)]
pub struct StoredValue {
    pub node_id: String,
    pub metric_id: String,
    pub value: f64,
}

impl LocalStore {
    pub fn open_default() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let directories = ProjectDirs::from("com", "Polygram", "Polygram").ok_or_else(|| {
            std::io::Error::other("Не удалось определить папку данных приложения")
        })?;
        let directory = directories.data_local_dir();
        fs::create_dir_all(directory)?;
        let path = directory.join(DATABASE_FILE);
        Ok(Self::open(&path)?)
    }

    fn open(path: &Path) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             CREATE TABLE IF NOT EXISTS metric_history (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 project_id TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 metric_id TEXT NOT NULL,
                 value REAL NOT NULL,
                 recorded_at_ms INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS metric_history_project_time
                 ON metric_history(project_id, recorded_at_ms);
             CREATE TABLE IF NOT EXISTS metric_latest (
                 project_id TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 metric_id TEXT NOT NULL,
                 value REAL NOT NULL,
                 updated_at_ms INTEGER NOT NULL,
                 PRIMARY KEY(project_id, node_id, metric_id)
             );",
        )?;
        Ok(Self { connection })
    }

    pub fn load_latest(&self, project: &Project) -> rusqlite::Result<Vec<StoredValue>> {
        let mut statement = self.connection.prepare(
            "SELECT node_id, metric_id, value
             FROM metric_latest
             WHERE project_id = ?1",
        )?;
        let values = statement
            .query_map([&project.storage_id], |row| {
                Ok(StoredValue {
                    node_id: row.get(0)?,
                    metric_id: row.get(1)?,
                    value: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(values)
    }

    pub fn store_ticks(&mut self, project_id: &str, ticks: &[Tick]) -> rusqlite::Result<()> {
        if ticks.is_empty() {
            return Ok(());
        }

        let recorded_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as i64;
        let transaction = self.connection.transaction()?;
        {
            let mut history = transaction.prepare_cached(
                "INSERT INTO metric_history
                    (project_id, node_id, metric_id, value, recorded_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            let mut latest = transaction.prepare_cached(
                "INSERT INTO metric_latest
                    (project_id, node_id, metric_id, value, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(project_id, node_id, metric_id)
                 DO UPDATE SET value = excluded.value, updated_at_ms = excluded.updated_at_ms",
            )?;
            for tick in ticks {
                history.execute(params![
                    project_id,
                    tick.node_id,
                    tick.metric_id,
                    tick.value,
                    recorded_at_ms
                ])?;
                latest.execute(params![
                    project_id,
                    tick.node_id,
                    tick.metric_id,
                    tick.value,
                    recorded_at_ms
                ])?;
            }
        }
        transaction.commit()
    }

    #[cfg(test)]
    fn in_memory() -> rusqlite::Result<Self> {
        Self::open(Path::new(":memory:"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(node_id: &str, metric_id: &str, value: f64) -> Tick {
        Tick {
            node_id: node_id.to_owned(),
            metric_id: metric_id.to_owned(),
            value,
        }
    }

    #[test]
    fn stores_history_and_restores_latest_values_per_project() {
        let mut store = LocalStore::in_memory().expect("store opens");
        let mut project = Project::default();
        project.storage_id = "project-a".to_owned();

        store
            .store_ticks(
                &project.storage_id,
                &[
                    tick("node-1", "metric-1", 10.0),
                    tick("node-1", "metric-1", 25.0),
                ],
            )
            .expect("ticks persist");
        store
            .store_ticks("project-b", &[tick("node-1", "metric-1", 99.0)])
            .expect("other project persists independently");

        assert_eq!(
            store.load_latest(&project).expect("latest values load"),
            vec![StoredValue {
                node_id: "node-1".to_owned(),
                metric_id: "metric-1".to_owned(),
                value: 25.0,
            }]
        );
        let history_count: i64 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM metric_history WHERE project_id = ?1",
                [&project.storage_id],
                |row| row.get(0),
            )
            .expect("history count loads");
        assert_eq!(history_count, 2);
    }

    #[test]
    fn empty_batch_is_a_no_op() {
        let mut store = LocalStore::in_memory().expect("store opens");
        store
            .store_ticks("project", &[])
            .expect("empty batch succeeds");
        let count: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM metric_history", [], |row| row.get(0))
            .expect("history count loads");
        assert_eq!(count, 0);
    }
}

use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Local job metadata. This journal does not execute or replay tools.
pub struct Journal(Connection);

impl Journal {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            CREATE TABLE IF NOT EXISTS jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                goal TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id INTEGER NOT NULL REFERENCES jobs(id),
                payload TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        Ok(Self(db))
    }

    pub fn create(&self, goal: &str) -> Result<i64> {
        if goal.trim().is_empty() {
            return Err("goal must not be empty".into());
        }
        self.0
            .execute("INSERT INTO jobs(goal) VALUES (?1)", [goal])?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn checkpoint(&self, job: i64, payload: &str) -> Result<i64> {
        let value: serde_json::Value = serde_json::from_str(payload)?;
        self.0.execute(
            "INSERT INTO checkpoints(job_id, payload) VALUES (?1, ?2)",
            params![job, value.to_string()],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn inspect(&self, job: i64) -> Result<serde_json::Value> {
        let goal: Option<String> = self
            .0
            .query_row("SELECT goal FROM jobs WHERE id = ?1", [job], |r| r.get(0))
            .optional()?;
        let goal = goal.ok_or("job does not exist")?;
        let payload: Option<String> = self
            .0
            .query_row(
                "SELECT payload FROM checkpoints WHERE job_id = ?1 ORDER BY id DESC LIMIT 1",
                [job],
                |r| r.get(0),
            )
            .optional()?;
        let checkpoint = payload
            .map(|p| serde_json::from_str::<serde_json::Value>(&p))
            .transpose()?;
        Ok(serde_json::json!({"id": job, "goal": goal, "checkpoint": checkpoint}))
    }

    pub fn list(&self) -> Result<Vec<(i64, String)>> {
        let mut stmt = self.0.prepare("SELECT id, goal FROM jobs ORDER BY id")?;
        Ok(stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?)
    }
}

//! Persistent orchestration state for v2 (`.rembrandt/state.db`).

use crate::isolation::IsolationMode;
use crate::{RembrandtError, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

/// Persisted session status for v2 orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Starting,
    Active,
    Idle,
    Completed,
    Failed,
    Stopped,
}

impl SessionStatus {
    fn as_str(self) -> &'static str {
        match self {
            SessionStatus::Starting => "starting",
            SessionStatus::Active => "active",
            SessionStatus::Idle => "idle",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Stopped => "stopped",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "starting" => Ok(SessionStatus::Starting),
            "active" => Ok(SessionStatus::Active),
            "idle" => Ok(SessionStatus::Idle),
            "completed" => Ok(SessionStatus::Completed),
            "failed" => Ok(SessionStatus::Failed),
            "stopped" => Ok(SessionStatus::Stopped),
            other => Err(RembrandtError::State(format!(
                "unknown session status '{}'",
                other
            ))),
        }
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Persisted v2 session record.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub agent_id: String,
    pub runtime_kind: String,
    pub runtime_session_id: Option<String>,
    pub isolation_mode: IsolationMode,
    pub branch_name: String,
    pub checkout_path: PathBuf,
    pub task_id: Option<String>,
    pub status: SessionStatus,
    pub model: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SQLite-backed state store.
pub struct StateStore {
    db_path: PathBuf,
    conn: Connection,
}

impl StateStore {
    pub fn open(repo_path: impl AsRef<Path>) -> Result<Self> {
        let rembrandt_dir = repo_path.as_ref().join(".rembrandt");
        std::fs::create_dir_all(&rembrandt_dir)?;
        let db_path = rembrandt_dir.join("state.db");
        let conn = Connection::open(&db_path)?;

        let mut store = Self { db_path, conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn init_schema(&mut self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS schema_migrations (
              version INTEGER PRIMARY KEY,
              applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
              agent_id TEXT PRIMARY KEY,
              runtime_kind TEXT NOT NULL,
              runtime_session_id TEXT,
              isolation_mode TEXT NOT NULL,
              branch_name TEXT NOT NULL,
              checkout_path TEXT NOT NULL,
              task_id TEXT,
              status TEXT NOT NULL,
              model TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS file_claims (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              agent_id TEXT NOT NULL,
              path TEXT NOT NULL,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS heartbeats (
              agent_id TEXT PRIMARY KEY,
              last_seen_at TEXT NOT NULL,
              detail TEXT
            );

            CREATE TABLE IF NOT EXISTS csi_runs (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              started_at TEXT NOT NULL,
              completed_at TEXT,
              status TEXT NOT NULL,
              summary TEXT
            );

            CREATE TABLE IF NOT EXISTS csi_events (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              csi_run_id INTEGER,
              agent_id TEXT,
              kind TEXT NOT NULL,
              message TEXT NOT NULL,
              created_at TEXT NOT NULL
            );
            "#,
        )?;

        self.conn.execute(
            "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES(1, ?1)",
            [Utc::now().to_rfc3339()],
        )?;

        Ok(())
    }

    pub fn upsert_session(&self, record: &SessionRecord) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO sessions (
              agent_id, runtime_kind, runtime_session_id, isolation_mode, branch_name,
              checkout_path, task_id, status, model, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(agent_id) DO UPDATE SET
              runtime_kind = excluded.runtime_kind,
              runtime_session_id = excluded.runtime_session_id,
              isolation_mode = excluded.isolation_mode,
              branch_name = excluded.branch_name,
              checkout_path = excluded.checkout_path,
              task_id = excluded.task_id,
              status = excluded.status,
              model = excluded.model,
              updated_at = excluded.updated_at
            "#,
            params![
                record.agent_id,
                record.runtime_kind,
                record.runtime_session_id,
                isolation_mode_to_str(record.isolation_mode),
                record.branch_name,
                record.checkout_path.to_string_lossy().to_string(),
                record.task_id,
                record.status.as_str(),
                record.model,
                record.created_at.to_rfc3339(),
                record.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn get_session(&self, agent_id: &str) -> Result<Option<SessionRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT agent_id, runtime_kind, runtime_session_id, isolation_mode, branch_name,
                   checkout_path, task_id, status, model, created_at, updated_at
            FROM sessions WHERE agent_id = ?1
            "#,
        )?;

        let row = stmt
            .query_row([agent_id], |row| {
                let created_at: String = row.get(9)?;
                let updated_at: String = row.get(10)?;
                Ok(SessionRecord {
                    agent_id: row.get(0)?,
                    runtime_kind: row.get(1)?,
                    runtime_session_id: row.get(2)?,
                    isolation_mode: isolation_mode_from_str(&row.get::<_, String>(3)?)
                        .map_err(to_sql_err)?,
                    branch_name: row.get(4)?,
                    checkout_path: PathBuf::from(row.get::<_, String>(5)?),
                    task_id: row.get(6)?,
                    status: SessionStatus::from_str(&row.get::<_, String>(7)?).map_err(to_sql_err)?,
                    model: row.get(8)?,
                    created_at: parse_rfc3339(&created_at).map_err(to_sql_err)?,
                    updated_at: parse_rfc3339(&updated_at).map_err(to_sql_err)?,
                })
            })
            .optional()?;

        Ok(row)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT agent_id, runtime_kind, runtime_session_id, isolation_mode, branch_name,
                   checkout_path, task_id, status, model, created_at, updated_at
            FROM sessions
            ORDER BY updated_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let created_at: String = row.get(9)?;
            let updated_at: String = row.get(10)?;
            Ok(SessionRecord {
                agent_id: row.get(0)?,
                runtime_kind: row.get(1)?,
                runtime_session_id: row.get(2)?,
                isolation_mode: isolation_mode_from_str(&row.get::<_, String>(3)?)
                    .map_err(to_sql_err)?,
                branch_name: row.get(4)?,
                checkout_path: PathBuf::from(row.get::<_, String>(5)?),
                task_id: row.get(6)?,
                status: SessionStatus::from_str(&row.get::<_, String>(7)?).map_err(to_sql_err)?,
                model: row.get(8)?,
                created_at: parse_rfc3339(&created_at).map_err(to_sql_err)?,
                updated_at: parse_rfc3339(&updated_at).map_err(to_sql_err)?,
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn update_status(&self, agent_id: &str, status: SessionStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE agent_id = ?3",
            params![status.as_str(), Utc::now().to_rfc3339(), agent_id],
        )?;
        Ok(())
    }

    pub fn touch_heartbeat(&self, agent_id: &str, detail: Option<&str>) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO heartbeats(agent_id, last_seen_at, detail) VALUES (?1, ?2, ?3)
            ON CONFLICT(agent_id) DO UPDATE SET
              last_seen_at = excluded.last_seen_at,
              detail = excluded.detail
            "#,
            params![agent_id, Utc::now().to_rfc3339(), detail],
        )?;
        Ok(())
    }
}

fn isolation_mode_to_str(mode: IsolationMode) -> &'static str {
    match mode {
        IsolationMode::Branch => "branch",
        IsolationMode::Worktree => "worktree",
    }
}

fn isolation_mode_from_str(value: &str) -> Result<IsolationMode> {
    match value {
        "branch" => Ok(IsolationMode::Branch),
        "worktree" => Ok(IsolationMode::Worktree),
        other => Err(RembrandtError::State(format!(
            "unknown isolation mode '{}'",
            other
        ))),
    }
}

fn parse_rfc3339(value: &str) -> Result<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| RembrandtError::State(format!("invalid timestamp '{}': {}", value, e)))
}

fn to_sql_err(err: RembrandtError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(err),
    )
}

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, params};

use crate::protocol::{ArchiveFormat, BuildView, RunView};

#[derive(Clone)]
pub struct Storage {
    inner: Arc<StorageInner>,
}

struct StorageInner {
    conn: Mutex<Connection>,
    sources_dir: PathBuf,
    logs_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BuildRow {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct RunRow {
    pub id: String,
    pub build_id: String,
    pub agent_name: String,
    pub labels: Vec<String>,
    pub status: String,
    pub source_path: PathBuf,
    pub archive_format: ArchiveFormat,
    pub script_timeout_sec: u64,
}

#[derive(Debug, Clone)]
pub struct NewRun {
    pub id: String,
    pub agent_name: String,
    pub labels: Vec<String>,
    pub script_timeout_sec: u64,
}

impl Storage {
    pub fn open(data_dir: PathBuf, db_path: PathBuf) -> anyhow::Result<Self> {
        let sources_dir = data_dir.join("sources");
        let logs_dir = data_dir.join("logs");
        let tmp_dir = data_dir.join("tmp");
        fs::create_dir_all(&sources_dir)
            .with_context(|| format!("create {}", sources_dir.display()))?;
        fs::create_dir_all(&logs_dir).with_context(|| format!("create {}", logs_dir.display()))?;
        fs::create_dir_all(&tmp_dir).with_context(|| format!("create {}", tmp_dir.display()))?;
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }

        let conn =
            Connection::open(&db_path).with_context(|| format!("open {}", db_path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        init_schema(&conn)?;

        Ok(Self {
            inner: Arc::new(StorageInner {
                conn: Mutex::new(conn),
                sources_dir,
                logs_dir,
            }),
        })
    }

    pub fn sources_dir(&self) -> &Path {
        &self.inner.sources_dir
    }

    pub fn log_path(&self, run_id: &str) -> PathBuf {
        self.inner.logs_dir.join(format!("{run_id}.log"))
    }

    pub fn create_build(
        &self,
        id: &str,
        source_name: &str,
        archive_format: ArchiveFormat,
        source_path: &Path,
        runs: &[NewRun],
    ) -> anyhow::Result<()> {
        let created_at = now_ts();
        let labels_json = |labels: &[String]| serde_json::to_string(labels).unwrap_or_default();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO builds(id, source_name, archive_format, source_path, created_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'queued')",
            params![
                id,
                source_name,
                archive_format.to_string(),
                source_path.to_string_lossy(),
                created_at
            ],
        )?;

        for run in runs {
            tx.execute(
                "INSERT INTO runs(
                    id, build_id, agent_name, labels_json, status, exit_code, created_at,
                    started_at, finished_at, source_path, archive_format, script_timeout_sec, rerun_of
                 )
                 VALUES (?1, ?2, ?3, ?4, 'queued', NULL, ?5, NULL, NULL, ?6, ?7, ?8, NULL)",
                params![
                    run.id,
                    id,
                    run.agent_name,
                    labels_json(&run.labels),
                    created_at,
                    source_path.to_string_lossy(),
                    archive_format.to_string(),
                    run.script_timeout_sec as i64,
                ],
            )?;
        }
        tx.commit()?;
        drop(conn);
        self.refresh_build_status(id)?;
        Ok(())
    }

    pub fn create_rerun(&self, source: &RunRow, new_run_id: &str) -> anyhow::Result<()> {
        let created_at = now_ts();
        let labels_json = serde_json::to_string(&source.labels)?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO runs(
                id, build_id, agent_name, labels_json, status, exit_code, created_at,
                started_at, finished_at, source_path, archive_format, script_timeout_sec, rerun_of
             )
             VALUES (?1, ?2, ?3, ?4, 'queued', NULL, ?5, NULL, NULL, ?6, ?7, ?8, ?9)",
            params![
                new_run_id,
                source.build_id,
                source.agent_name,
                labels_json,
                created_at,
                source.source_path.to_string_lossy(),
                source.archive_format.to_string(),
                source.script_timeout_sec as i64,
                source.id,
            ],
        )?;
        drop(conn);
        self.refresh_build_status(&source.build_id)?;
        Ok(())
    }

    pub fn list_builds(&self) -> anyhow::Result<Vec<BuildView>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, source_name, archive_format, status, created_at
             FROM builds ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BuildView {
                id: row.get(0)?,
                source_name: row.get(1)?,
                archive_format: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn get_build(&self, build_id: &str) -> anyhow::Result<Option<BuildRow>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT id FROM builds WHERE id = ?1")?;
        stmt.query_row(params![build_id], |row| Ok(BuildRow { id: row.get(0)? }))
            .optional()
            .map_err(Into::into)
    }

    pub fn list_runs(&self) -> anyhow::Result<Vec<RunView>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, build_id, agent_name, labels_json, status, exit_code, created_at,
                    started_at, finished_at
             FROM runs ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let labels_json: String = row.get(3)?;
            Ok(RunView {
                id: row.get(0)?,
                build_id: row.get(1)?,
                agent_name: row.get(2)?,
                labels: parse_labels(&labels_json),
                status: row.get(4)?,
                exit_code: row.get(5)?,
                created_at: row.get(6)?,
                started_at: row.get(7)?,
                finished_at: row.get(8)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn queued_runs(&self) -> anyhow::Result<Vec<RunRow>> {
        self.runs_by_status("queued")
    }

    pub fn get_run(&self, run_id: &str) -> anyhow::Result<Option<RunRow>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, build_id, agent_name, labels_json, status, exit_code, created_at,
                    started_at, finished_at, source_path, archive_format, script_timeout_sec
             FROM runs WHERE id = ?1",
        )?;
        stmt.query_row(params![run_id], row_to_run)
            .optional()
            .map_err(Into::into)
    }

    pub fn update_run_status(&self, run_id: &str, status: &str) -> anyhow::Result<()> {
        let now = now_ts();
        let conn = self.conn()?;
        match status {
            "preparing" | "running" => {
                conn.execute(
                    "UPDATE runs
                     SET status = ?2, started_at = COALESCE(started_at, ?3)
                     WHERE id = ?1",
                    params![run_id, status, now],
                )?;
            }
            "success" | "failed" | "canceled" | "lost" => {
                conn.execute(
                    "UPDATE runs
                     SET status = ?2, finished_at = COALESCE(finished_at, ?3)
                     WHERE id = ?1",
                    params![run_id, status, now],
                )?;
            }
            _ => {
                conn.execute(
                    "UPDATE runs SET status = ?2 WHERE id = ?1",
                    params![run_id, status],
                )?;
            }
        }
        drop(conn);
        if let Some(run) = self.get_run(run_id)? {
            self.refresh_build_status(&run.build_id)?;
        }
        Ok(())
    }

    pub fn finish_run(&self, run_id: &str, exit_code: i32) -> anyhow::Result<()> {
        let status = if exit_code == 0 { "success" } else { "failed" };
        let now = now_ts();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE runs
             SET status = ?2, exit_code = ?3, finished_at = COALESCE(finished_at, ?4)
             WHERE id = ?1",
            params![run_id, status, exit_code, now],
        )?;
        drop(conn);
        if let Some(run) = self.get_run(run_id)? {
            self.refresh_build_status(&run.build_id)?;
        }
        Ok(())
    }

    pub fn mark_active_runs_lost(&self) -> anyhow::Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id FROM runs
             WHERE status IN ('assigned', 'preparing', 'running')",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let ids = collect_rows(rows)?;
        drop(stmt);
        for id in &ids {
            self.update_run_status(id, "lost")?;
        }
        Ok(ids)
    }

    pub fn mark_agent_runs_lost(&self, agent_name: &str) -> anyhow::Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id FROM runs
             WHERE agent_name = ?1 AND status IN ('assigned', 'preparing', 'running')",
        )?;
        let rows = stmt.query_map(params![agent_name], |row| row.get::<_, String>(0))?;
        let ids = collect_rows(rows)?;
        drop(stmt);
        for id in &ids {
            self.update_run_status(id, "lost")?;
        }
        Ok(ids)
    }

    pub fn mark_run_assigned(&self, run_id: &str) -> anyhow::Result<()> {
        self.update_run_status(run_id, "assigned")
    }

    pub fn cancel_run(&self, run_id: &str) -> anyhow::Result<()> {
        self.update_run_status(run_id, "canceled")
    }

    pub fn delete_run(&self, run_id: &str) -> anyhow::Result<()> {
        let run = self.get_run(run_id)?.context("run not found")?;
        let conn = self.conn()?;
        conn.execute("DELETE FROM runs WHERE id = ?1", params![run_id])?;
        drop(conn);

        let path = self.log_path(run_id);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err).with_context(|| format!("remove {}", path.display())),
        }

        self.refresh_build_status(&run.build_id)?;
        Ok(())
    }

    pub fn delete_build(&self, build_id: &str) -> anyhow::Result<()> {
        let build = self.get_build(build_id)?.context("build not found")?;
        let conn = self.conn()?;
        let run_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM runs WHERE build_id = ?1",
            params![build_id],
            |row| row.get(0),
        )?;
        if run_count > 0 {
            anyhow::bail!("delete runs for this build first");
        }
        drop(conn);

        let build_dir = build_source_dir(&self.inner.sources_dir, &build.id)?;
        match fs::remove_dir_all(&build_dir) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err).with_context(|| format!("remove {}", build_dir.display())),
        }

        let conn = self.conn()?;
        conn.execute("DELETE FROM builds WHERE id = ?1", params![build_id])?;
        Ok(())
    }

    pub fn append_log(&self, run_id: &str, stream: &str, bytes: &[u8]) -> anyhow::Result<()> {
        let path = self.log_path(run_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        if stream == "stderr" {
            file.write_all(b"[stderr] ")?;
        }
        file.write_all(bytes)?;
        Ok(())
    }

    pub fn read_log(&self, run_id: &str) -> anyhow::Result<String> {
        let path = self.log_path(run_id);
        match fs::read(&path) {
            Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(err) => Err(err).with_context(|| format!("read {}", path.display())),
        }
    }

    pub fn cleanup_old_logs(&self, retention_days: u64) -> anyhow::Result<()> {
        let cutoff = now_ts() - (retention_days as i64 * 24 * 60 * 60);
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id FROM runs
             WHERE finished_at IS NOT NULL AND finished_at < ?1",
        )?;
        let rows = stmt.query_map(params![cutoff], |row| row.get::<_, String>(0))?;
        let ids = collect_rows(rows)?;
        drop(stmt);
        for id in ids {
            let path = self.log_path(&id);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(err).with_context(|| format!("remove {}", path.display())),
            }
        }
        Ok(())
    }

    fn runs_by_status(&self, status: &str) -> anyhow::Result<Vec<RunRow>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, build_id, agent_name, labels_json, status, exit_code, created_at,
                    started_at, finished_at, source_path, archive_format, script_timeout_sec
             FROM runs WHERE status = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![status], row_to_run)?;
        collect_rows(rows)
    }

    fn refresh_build_status(&self, build_id: &str) -> anyhow::Result<()> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT status FROM runs WHERE build_id = ?1")?;
        let rows = stmt.query_map(params![build_id], |row| row.get::<_, String>(0))?;
        let statuses = collect_rows(rows)?;
        let build_status = aggregate_build_status(&statuses);
        conn.execute(
            "UPDATE builds SET status = ?2 WHERE id = ?1",
            params![build_id, build_status],
        )?;
        Ok(())
    }

    fn conn(&self) -> anyhow::Result<std::sync::MutexGuard<'_, Connection>> {
        self.inner
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("sqlite mutex poisoned"))
    }
}

fn init_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS builds (
            id TEXT PRIMARY KEY,
            source_name TEXT NOT NULL,
            archive_format TEXT NOT NULL,
            source_path TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            status TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS runs (
            id TEXT PRIMARY KEY,
            build_id TEXT NOT NULL REFERENCES builds(id),
            agent_name TEXT NOT NULL,
            labels_json TEXT NOT NULL,
            status TEXT NOT NULL,
            exit_code INTEGER,
            created_at INTEGER NOT NULL,
            started_at INTEGER,
            finished_at INTEGER,
            source_path TEXT NOT NULL,
            archive_format TEXT NOT NULL,
            script_timeout_sec INTEGER NOT NULL,
            rerun_of TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status);
        CREATE INDEX IF NOT EXISTS idx_runs_agent_status ON runs(agent_name, status);
        CREATE INDEX IF NOT EXISTS idx_runs_build_id ON runs(build_id);
        "#,
    )?;
    Ok(())
}

fn row_to_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunRow> {
    let labels_json: String = row.get(3)?;
    let archive_format: String = row.get(10)?;
    Ok(RunRow {
        id: row.get(0)?,
        build_id: row.get(1)?,
        agent_name: row.get(2)?,
        labels: parse_labels(&labels_json),
        status: row.get(4)?,
        source_path: PathBuf::from(row.get::<_, String>(9)?),
        archive_format: match archive_format.as_str() {
            "tar.gz" => ArchiveFormat::TarGz,
            "zip" => ArchiveFormat::Zip,
            _ => return Err(rusqlite::Error::InvalidQuery),
        },
        script_timeout_sec: row.get::<_, i64>(11)? as u64,
    })
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> anyhow::Result<Vec<T>> {
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

fn parse_labels(labels_json: &str) -> Vec<String> {
    serde_json::from_str(labels_json).unwrap_or_default()
}

fn aggregate_build_status(statuses: &[String]) -> &'static str {
    if statuses.is_empty() {
        return "queued";
    }
    if statuses
        .iter()
        .any(|s| matches!(s.as_str(), "queued" | "assigned" | "preparing" | "running"))
    {
        "running"
    } else if statuses.iter().all(|s| s == "success") {
        "success"
    } else if statuses.iter().any(|s| s == "canceled") {
        "canceled"
    } else if statuses.iter().any(|s| s == "lost") {
        "lost"
    } else {
        "failed"
    }
}

fn build_source_dir(sources_dir: &Path, build_id: &str) -> anyhow::Result<PathBuf> {
    if build_id.is_empty()
        || build_id == "."
        || build_id == ".."
        || build_id.contains('/')
        || build_id.contains('\\')
    {
        anyhow::bail!("invalid build id");
    }
    Ok(sources_dir.join(build_id))
}

pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_build_runs_reruns_and_logs() {
        let dir = tempfile::tempdir().unwrap();
        let storage =
            Storage::open(dir.path().to_path_buf(), dir.path().join("buildsvc.db")).unwrap();
        let source_dir = storage.sources_dir().join("build_1");
        fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("source.tar.gz");
        fs::write(&source, b"fake").unwrap();

        storage
            .create_build(
                "build_1",
                "source.tar.gz",
                ArchiveFormat::TarGz,
                &source,
                &[NewRun {
                    id: "run_1".to_owned(),
                    agent_name: "agent_1".to_owned(),
                    labels: vec!["linux".to_owned(), "amd64".to_owned()],
                    script_timeout_sec: 60,
                }],
            )
            .unwrap();

        let queued = storage.queued_runs().unwrap();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].id, "run_1");

        storage.mark_run_assigned("run_1").unwrap();
        storage.finish_run("run_1", 1).unwrap();
        let run = storage.get_run("run_1").unwrap().unwrap();
        assert_eq!(run.status, "failed");

        storage.create_rerun(&run, "run_2").unwrap();
        assert_eq!(storage.queued_runs().unwrap()[0].id, "run_2");

        storage.append_log("run_1", "stdout", b"hello\n").unwrap();
        assert_eq!(storage.read_log("run_1").unwrap(), "hello\n");

        storage.delete_run("run_1").unwrap();
        assert!(storage.get_run("run_1").unwrap().is_none());
        assert_eq!(storage.read_log("run_1").unwrap(), "");

        storage.delete_run("run_2").unwrap();
        storage.delete_build("build_1").unwrap();
        assert!(storage.get_build("build_1").unwrap().is_none());
        assert!(!source_dir.exists());
    }

    #[test]
    fn rejects_unsafe_build_source_dir_ids() {
        let sources_dir = Path::new("/tmp/buildsvc-sources");
        assert!(build_source_dir(sources_dir, "build_123").is_ok());
        assert!(build_source_dir(sources_dir, "../build_123").is_err());
        assert!(build_source_dir(sources_dir, "build/123").is_err());
        assert!(build_source_dir(sources_dir, r"build\123").is_err());
        assert!(build_source_dir(sources_dir, "").is_err());
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentToServer {
    Hello {
        name: String,
        token: String,
        computer_name: String,
        username: String,
        ip: String,
        labels: Vec<String>,
        platform: String,
        arch: String,
        concurrency: usize,
        version: String,
    },
    Heartbeat {
        running: usize,
        capacity: usize,
        runs: Vec<AgentRunSnapshot>,
    },
    RunStatus {
        run_id: String,
        state: String,
    },
    RunLog {
        run_id: String,
        stream: LogStream,
        seq: u64,
        data: String,
    },
    RunFinished {
        run_id: String,
        exit_code: i32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToAgent {
    HelloAccepted {
        heartbeat_sec: u64,
    },
    RunStart {
        run_id: String,
        build_id: String,
        source_url: String,
        archive_format: ArchiveFormat,
        script_timeout_sec: u64,
    },
    RunCancel {
        run_id: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat {
    TarGz,
    Zip,
}

impl ArchiveFormat {
    pub fn from_filename(filename: &str) -> Option<Self> {
        let lower = filename.to_ascii_lowercase();
        if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
            Some(Self::TarGz)
        } else if lower.ends_with(".zip") {
            Some(Self::Zip)
        } else {
            None
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::TarGz => "tar.gz",
            Self::Zip => "zip",
        }
    }
}

impl std::fmt::Display for ArchiveFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TarGz => f.write_str("tar.gz"),
            Self::Zip => f.write_str("zip"),
        }
    }
}

impl std::str::FromStr for ArchiveFormat {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "tar.gz" | "tgz" => Ok(Self::TarGz),
            "zip" => Ok(Self::Zip),
            other => anyhow::bail!("unsupported archive format: {other}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunSnapshot {
    pub run_id: String,
    pub state: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    Stdout,
    Stderr,
}

impl std::fmt::Display for LogStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdout => f.write_str("stdout"),
            Self::Stderr => f.write_str("stderr"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiMessage {
    State { state: UiState },
    Log { run_id: String, data: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct UiState {
    pub agents: Vec<AgentView>,
    pub builds: Vec<BuildView>,
    pub runs: Vec<RunView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentView {
    pub name: String,
    pub computer_name: Option<String>,
    pub username: Option<String>,
    pub ip: Option<String>,
    pub labels: Vec<String>,
    pub platform: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
    pub status: String,
    pub running: usize,
    pub capacity: usize,
    pub current_runs: Vec<String>,
    pub last_seen: Option<i64>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildView {
    pub id: String,
    pub source_name: String,
    pub archive_format: String,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunView {
    pub id: String,
    pub build_id: String,
    pub agent_name: String,
    pub labels: Vec<String>,
    pub status: String,
    pub exit_code: Option<i32>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_archive_format_from_filename() {
        assert_eq!(
            ArchiveFormat::from_filename("source.tar.gz"),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            ArchiveFormat::from_filename("source.tgz"),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            ArchiveFormat::from_filename("source.zip"),
            Some(ArchiveFormat::Zip)
        );
        assert_eq!(ArchiveFormat::from_filename("source.tar.xz"), None);
    }
}

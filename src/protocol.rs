use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentToServer {
    Hello {
        agent_id: String,
        token: String,
        computer_name: String,
        username: String,
        ip: String,
        platform: String,
        arch: String,
        concurrency: usize,
        terminal_enabled: bool,
        #[serde(default)]
        upgrade_enabled: bool,
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
    RunDeleted {
        run_id: String,
        success: bool,
        error: Option<String>,
    },
    TerminalStarted {
        session_id: String,
    },
    TerminalOutput {
        session_id: String,
        data: String,
    },
    TerminalExit {
        session_id: String,
        exit_code: Option<i32>,
        message: Option<String>,
    },
    UpgradeStatus {
        upgrade_id: String,
        state: String,
        message: Option<String>,
    },
    UpgradeLog {
        upgrade_id: String,
        stream: LogStream,
        seq: u64,
        data: String,
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
    RunDelete {
        run_id: String,
    },
    TerminalStart {
        session_id: String,
        rows: u16,
        cols: u16,
    },
    TerminalInput {
        session_id: String,
        data: String,
    },
    TerminalResize {
        session_id: String,
        rows: u16,
        cols: u16,
    },
    TerminalClose {
        session_id: String,
    },
    UpgradeStart {
        upgrade_id: String,
        package_url: String,
        package_kind: UpgradePackageKind,
        filename: String,
        sha256: String,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpgradePackageKind {
    Deb,
    Rpm,
    Emerge,
}

impl UpgradePackageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deb => "deb",
            Self::Rpm => "rpm",
            Self::Emerge => "emerge",
        }
    }
}

impl std::fmt::Display for UpgradePackageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for UpgradePackageKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "deb" => Ok(Self::Deb),
            "rpm" => Ok(Self::Rpm),
            "emerge" | "gentoo" => Ok(Self::Emerge),
            other => anyhow::bail!("unsupported upgrade package kind: {other}"),
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
    State {
        state: UiState,
    },
    Log {
        run_id: String,
        data: String,
    },
    UpgradeLog {
        agent_id: String,
        upgrade_id: String,
        data: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct UiState {
    pub agents: Vec<AgentView>,
    pub builds: Vec<BuildView>,
    pub runs: Vec<RunView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentView {
    pub id: String,
    pub computer_name: Option<String>,
    pub username: Option<String>,
    pub ip: Option<String>,
    pub platform: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
    pub status: String,
    pub running: usize,
    pub capacity: usize,
    pub current_runs: Vec<String>,
    pub last_seen: Option<i64>,
    pub terminal_enabled: bool,
    pub upgrade_enabled: bool,
    pub upgrade_status: Option<String>,
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
    pub agent_id: String,
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

    #[test]
    fn parses_upgrade_package_kind() {
        assert_eq!(
            "deb".parse::<UpgradePackageKind>().unwrap(),
            UpgradePackageKind::Deb
        );
        assert_eq!(
            "gentoo".parse::<UpgradePackageKind>().unwrap(),
            UpgradePackageKind::Emerge
        );
        assert!("zip".parse::<UpgradePackageKind>().is_err());
    }

    #[test]
    fn hello_defaults_upgrade_capability() {
        let message = format!(
            r#"{{
            "type": "hello",
            "agent_id": "agent_123",
            "token": "secret",
            "computer_name": "host",
            "username": "user",
            "ip": "192.168.1.2",
            "platform": "linux",
            "arch": "x86_64",
            "concurrency": 1,
            "terminal_enabled": false,
            "version": "{}"
        }}"#,
            env!("CARGO_PKG_VERSION")
        );
        let AgentToServer::Hello {
            upgrade_enabled, ..
        } = serde_json::from_str(&message).unwrap()
        else {
            panic!("expected hello");
        };
        assert!(!upgrade_enabled);
    }
}

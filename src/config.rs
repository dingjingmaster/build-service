use std::{
    collections::BTreeMap,
    env, fmt, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Server,
    Agent,
}

impl std::str::FromStr for Role {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "server" => Ok(Self::Server),
            "agent" => Ok(Self::Agent),
            other => bail!("unsupported role: {other}"),
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Server => f.write_str("server"),
            Self::Agent => f.write_str("agent"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CoreConfig {
    pub role: Role,
    pub data_dir: PathBuf,
    pub log_level: String,
}

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub listen: String,
    pub public_url: String,
    pub db_path: PathBuf,
    pub log_retention_days: u64,
    pub agent_offline_after_sec: u64,
    pub agent_heartbeat_sec: u64,
    pub script_timeout_sec: u64,
    pub kill_grace_sec: u64,
    pub max_upload_size_mb: u64,
    pub terminal_enabled: bool,
    pub upgrade_enabled: bool,
}

#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub server_url: String,
    pub name: String,
    pub token: String,
    pub advertise_ip: Option<String>,
    pub labels: Vec<String>,
    pub work_dir: PathBuf,
    pub concurrency: usize,
    pub heartbeat_sec: u64,
    pub script_timeout_sec: u64,
    pub kill_grace_sec: u64,
    pub terminal_enabled: bool,
    pub terminal_shell: Option<String>,
    pub terminal_work_dir: PathBuf,
    pub terminal_max_sessions: usize,
    pub upgrade_enabled: bool,
    pub upgrade_work_dir: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServerAgentConfig {
    pub name: String,
    #[serde(skip_serializing)]
    pub token: String,
    pub labels: Vec<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub core: CoreConfig,
    pub server: Option<ServerConfig>,
    pub agent: Option<AgentConfig>,
    pub server_agents: BTreeMap<String, ServerAgentConfig>,
}

#[derive(Debug, Clone)]
struct Ini {
    sections: BTreeMap<String, BTreeMap<String, String>>,
}

impl AppConfig {
    pub fn load(config_path: Option<&Path>) -> anyhow::Result<Self> {
        let path = match config_path {
            Some(path) => path.to_path_buf(),
            None => discover_config_path().context("discover config path")?,
        };
        let content = fs::read_to_string(&path)
            .with_context(|| format!("read config file {}", path.display()))?;
        Self::from_ini_str(&content).with_context(|| format!("parse {}", path.display()))
    }

    pub fn from_ini_str(content: &str) -> anyhow::Result<Self> {
        let ini = Ini::parse(content)?;
        let core_section = ini.section("core").context("missing [core] section")?;
        let role: Role = required(core_section, "role")?.parse()?;
        let data_dir = PathBuf::from(
            optional(core_section, "data_dir")
                .map(ToOwned::to_owned)
                .unwrap_or_else(default_data_dir),
        );
        let log_level = optional(core_section, "log_level")
            .unwrap_or("info")
            .to_owned();

        let core = CoreConfig {
            role,
            data_dir,
            log_level,
        };

        let server = ini
            .section("server")
            .map(|section| parse_server_config(&core, section))
            .transpose()?;
        let agent = ini
            .section("agent")
            .map(|section| parse_agent_config(&core, section))
            .transpose()?;

        let mut server_agents = BTreeMap::new();
        for (section_name, section) in &ini.sections {
            if let Some(agent_name) = section_name.strip_prefix("agent.") {
                let enabled = parse_bool(optional(section, "enabled").unwrap_or("true"))?;
                let labels = parse_list(optional(section, "labels").unwrap_or(""));
                let token = required(section, "token")?.to_owned();
                server_agents.insert(
                    agent_name.to_owned(),
                    ServerAgentConfig {
                        name: agent_name.to_owned(),
                        token,
                        labels,
                        enabled,
                    },
                );
            }
        }

        Ok(Self {
            core,
            server,
            agent,
            server_agents,
        })
    }
}

pub fn discover_config_path() -> anyhow::Result<PathBuf> {
    let candidates = config_candidates();
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!("no buildsvc.ini found; checked ./buildsvc.ini and platform service path");
}

fn config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        paths.push(PathBuf::from(r"C:\ProgramData\buildsvc\buildsvc.ini"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        paths.push(PathBuf::from("/etc/buildsvc/buildsvc.ini"));
    }

    if let Ok(current_dir) = env::current_dir() {
        paths.push(current_dir.join("buildsvc.ini"));
    }

    paths
}

fn default_data_dir() -> String {
    #[cfg(target_os = "windows")]
    {
        r"C:\ProgramData\buildsvc\data".to_owned()
    }
    #[cfg(not(target_os = "windows"))]
    {
        "/var/lib/buildsvc".to_owned()
    }
}

fn parse_server_config(
    core: &CoreConfig,
    section: &BTreeMap<String, String>,
) -> anyhow::Result<ServerConfig> {
    let db_path = optional(section, "db_path")
        .map(PathBuf::from)
        .unwrap_or_else(|| core.data_dir.join("buildsvc.db"));

    Ok(ServerConfig {
        listen: optional(section, "listen")
            .unwrap_or("0.0.0.0:8080")
            .to_owned(),
        public_url: optional(section, "public_url")
            .unwrap_or("http://127.0.0.1:8080")
            .trim_end_matches('/')
            .to_owned(),
        db_path,
        log_retention_days: parse_u64(section, "log_retention_days", 7)?,
        agent_offline_after_sec: parse_u64(section, "agent_offline_after_sec", 15)?,
        agent_heartbeat_sec: parse_u64(section, "agent_heartbeat_sec", 5)?,
        script_timeout_sec: parse_u64(section, "script_timeout_sec", 7200)?,
        kill_grace_sec: parse_u64(section, "kill_grace_sec", 10)?,
        max_upload_size_mb: parse_u64(section, "max_upload_size_mb", 2048)?,
        terminal_enabled: parse_bool(optional(section, "terminal_enabled").unwrap_or("false"))?,
        upgrade_enabled: parse_bool(optional(section, "upgrade_enabled").unwrap_or("false"))?,
    })
}

fn parse_agent_config(
    core: &CoreConfig,
    section: &BTreeMap<String, String>,
) -> anyhow::Result<AgentConfig> {
    let labels = parse_list(optional(section, "labels").unwrap_or(""));
    if labels.is_empty() {
        bail!("[agent].labels must not be empty");
    }

    let work_dir = optional(section, "work_dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| core.data_dir.join("work"));

    Ok(AgentConfig {
        server_url: required(section, "server_url")?.to_owned(),
        name: required(section, "name")?.to_owned(),
        token: required(section, "token")?.to_owned(),
        advertise_ip: optional(section, "advertise_ip").map(ToOwned::to_owned),
        labels,
        work_dir: work_dir.clone(),
        concurrency: parse_usize(section, "concurrency", 1)?.max(1),
        heartbeat_sec: parse_u64(section, "heartbeat_sec", 5)?,
        script_timeout_sec: parse_u64(section, "script_timeout_sec", 7200)?,
        kill_grace_sec: parse_u64(section, "kill_grace_sec", 10)?,
        terminal_enabled: parse_bool(optional(section, "terminal_enabled").unwrap_or("false"))?,
        terminal_shell: optional(section, "terminal_shell").map(ToOwned::to_owned),
        terminal_work_dir: optional(section, "terminal_work_dir")
            .map(PathBuf::from)
            .unwrap_or_else(|| work_dir.join("terminal")),
        terminal_max_sessions: parse_usize(section, "terminal_max_sessions", 1)?.max(1),
        upgrade_enabled: parse_bool(optional(section, "upgrade_enabled").unwrap_or("false"))?,
        upgrade_work_dir: optional(section, "upgrade_work_dir")
            .map(PathBuf::from)
            .unwrap_or_else(|| work_dir.join("upgrades")),
    })
}

fn required<'a>(section: &'a BTreeMap<String, String>, key: &str) -> anyhow::Result<&'a str> {
    section
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .with_context(|| format!("missing required key {key}"))
}

fn optional<'a>(section: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    section
        .get(key)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_u64(section: &BTreeMap<String, String>, key: &str, default: u64) -> anyhow::Result<u64> {
    optional(section, key)
        .map(|value| {
            value
                .parse::<u64>()
                .with_context(|| format!("invalid integer {key}={value}"))
        })
        .unwrap_or(Ok(default))
}

fn parse_usize(
    section: &BTreeMap<String, String>,
    key: &str,
    default: usize,
) -> anyhow::Result<usize> {
    optional(section, key)
        .map(|value| {
            value
                .parse::<usize>()
                .with_context(|| format!("invalid integer {key}={value}"))
        })
        .unwrap_or(Ok(default))
}

fn parse_bool(value: &str) -> anyhow::Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Ok(true),
        "false" | "no" | "0" | "off" => Ok(false),
        other => bail!("invalid boolean: {other}"),
    }
}

fn parse_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

impl Ini {
    fn parse(content: &str) -> anyhow::Result<Self> {
        let mut sections: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut current = String::new();

        for (idx, raw_line) in content.lines().enumerate() {
            let line_no = idx + 1;
            let line = strip_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }

            if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                let name = section.trim();
                if name.is_empty() {
                    bail!("empty section name at line {line_no}");
                }
                current = name.to_ascii_lowercase();
                sections.entry(current.clone()).or_default();
                continue;
            }

            if current.is_empty() {
                bail!("key outside section at line {line_no}");
            }

            let Some((key, value)) = line.split_once('=') else {
                bail!("invalid ini line {line_no}: {raw_line}");
            };
            let key = key.trim().to_ascii_lowercase();
            if key.is_empty() {
                bail!("empty key at line {line_no}");
            }
            sections
                .entry(current.clone())
                .or_default()
                .insert(key, unquote(value.trim()).to_owned());
        }

        Ok(Self { sections })
    }

    fn section(&self, name: &str) -> Option<&BTreeMap<String, String>> {
        self.sections.get(&name.to_ascii_lowercase())
    }
}

fn strip_comment(line: &str) -> &str {
    let semicolon = line.find(';');
    let hash = line.find('#');
    match (semicolon, hash) {
        (Some(a), Some(b)) => &line[..a.min(b)],
        (Some(a), None) => &line[..a],
        (None, Some(b)) => &line[..b],
        (None, None) => line,
    }
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
}

#[allow(dead_code)]
fn _assert_path_is_send_sync(_: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_server_and_agent_sections() {
        let config = AppConfig::from_ini_str(
            r#"
            [core]
            role = server
            data_dir = ./data

            [server]
            listen = 127.0.0.1:9090

            [agent.builder-1]
            token = secret
            labels = linux, amd64
            enabled = yes
            "#,
        )
        .unwrap();

        assert_eq!(config.core.role, Role::Server);
        assert_eq!(config.core.data_dir, PathBuf::from("./data"));
        let server = config.server.unwrap();
        assert_eq!(server.listen, "127.0.0.1:9090");
        assert!(!server.upgrade_enabled);
        assert_eq!(
            config.server_agents["builder-1"].labels,
            vec!["linux", "amd64"]
        );
    }

    #[test]
    fn parses_agent_defaults() {
        let config = AppConfig::from_ini_str(
            r#"
            [core]
            role = agent
            data_dir = /tmp/buildsvc-agent

            [agent]
            server_url = ws://127.0.0.1:8080/api/agent/ws
            name = local
            token = secret
            labels = linux,amd64
            "#,
        )
        .unwrap();

        let agent = config.agent.unwrap();
        assert_eq!(agent.name, "local");
        assert_eq!(agent.work_dir, PathBuf::from("/tmp/buildsvc-agent/work"));
        assert_eq!(agent.concurrency, 1);
        assert_eq!(agent.heartbeat_sec, 5);
        assert!(!agent.terminal_enabled);
        assert_eq!(
            agent.terminal_work_dir,
            PathBuf::from("/tmp/buildsvc-agent/work/terminal")
        );
        assert_eq!(agent.terminal_max_sessions, 1);
        assert!(!agent.upgrade_enabled);
        assert_eq!(
            agent.upgrade_work_dir,
            PathBuf::from("/tmp/buildsvc-agent/work/upgrades")
        );
    }
}

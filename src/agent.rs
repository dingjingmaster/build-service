use std::{
    collections::HashMap,
    io::{Read, Write},
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use flate2::read::GzDecoder;
use futures_util::{SinkExt, StreamExt};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::{
    fs,
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
    sync::{Mutex, Semaphore, mpsc, oneshot},
    time,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    archive,
    config::{AgentConfig, CoreConfig},
    protocol::{
        AgentRunSnapshot, AgentToServer, ArchiveFormat, LogStream, ServerToAgent,
        UpgradePackageKind,
    },
};

#[derive(Clone)]
struct AgentRuntime {
    config: AgentConfig,
    identity: AgentIdentity,
    client: Client,
    sender: mpsc::UnboundedSender<AgentToServer>,
    runs: Arc<Mutex<HashMap<String, RunControl>>>,
    terminal_sessions: Arc<Mutex<HashMap<String, TerminalControl>>>,
    upgrade: Arc<Mutex<Option<String>>>,
    semaphore: Arc<Semaphore>,
}

#[derive(Clone)]
struct AgentIdentity {
    id: String,
    token: String,
}

struct RunControl {
    state: String,
    cancel: oneshot::Sender<()>,
}

struct TerminalControl {
    sender: mpsc::UnboundedSender<TerminalCommand>,
}

enum TerminalCommand {
    Input(Vec<u8>),
    Resize { rows: u16, cols: u16 },
    Close,
}

pub async fn run(core: CoreConfig, config: AgentConfig) -> anyhow::Result<()> {
    fs::create_dir_all(&core.data_dir)
        .await
        .with_context(|| format!("create {}", core.data_dir.display()))?;
    let identity = load_or_create_agent_identity(&core.data_dir).await?;
    fs::create_dir_all(&config.work_dir)
        .await
        .with_context(|| format!("create {}", config.work_dir.display()))?;
    if config.terminal_enabled {
        fs::create_dir_all(&config.terminal_work_dir)
            .await
            .with_context(|| format!("create {}", config.terminal_work_dir.display()))?;
    }
    if config.upgrade_enabled {
        fs::create_dir_all(&config.upgrade_work_dir)
            .await
            .with_context(|| format!("create {}", config.upgrade_work_dir.display()))?;
    }
    fs::create_dir_all(core.data_dir.join("tmp")).await.ok();

    loop {
        match run_once(config.clone(), identity.clone()).await {
            Ok(()) => warn!("agent websocket closed"),
            Err(err) => warn!(%err, "agent websocket error"),
        }
        time::sleep(Duration::from_secs(3)).await;
    }
}

async fn run_once(config: AgentConfig, identity: AgentIdentity) -> anyhow::Result<()> {
    let (ws, _) = connect_async(&config.server_url)
        .await
        .with_context(|| format!("connect {}", config.server_url))?;
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<AgentToServer>();
    let runtime = AgentRuntime {
        config: config.clone(),
        identity: identity.clone(),
        client: Client::new(),
        sender: out_tx.clone(),
        runs: Arc::new(Mutex::new(HashMap::new())),
        terminal_sessions: Arc::new(Mutex::new(HashMap::new())),
        upgrade: Arc::new(Mutex::new(None)),
        semaphore: Arc::new(Semaphore::new(config.concurrency.max(1))),
    };

    out_tx.send(AgentToServer::Hello {
        agent_id: identity.id.clone(),
        token: identity.token.clone(),
        computer_name: computer_name(),
        username: username(),
        ip: advertised_ip(&config),
        platform: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        concurrency: config.concurrency.max(1),
        terminal_enabled: config.terminal_enabled,
        upgrade_enabled: config.upgrade_enabled,
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })?;

    let writer = tokio::spawn(async move {
        while let Some(message) = out_rx.recv().await {
            let Ok(text) = serde_json::to_string(&message) else {
                continue;
            };
            if ws_tx.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    let mut heartbeat = None;

    while let Some(message) = ws_rx.next().await {
        let message = message?;
        match message {
            Message::Text(text) => {
                let server_message: ServerToAgent = serde_json::from_str(&text)?;
                match server_message {
                    ServerToAgent::HelloAccepted { heartbeat_sec } => {
                        let heartbeat_sec = if heartbeat_sec == 0 {
                            runtime.config.heartbeat_sec
                        } else {
                            heartbeat_sec
                        };
                        info!(heartbeat_sec, "agent connected");
                        if heartbeat.is_none() {
                            let heartbeat_runtime = runtime.clone();
                            heartbeat = Some(tokio::spawn(async move {
                                heartbeat_loop(heartbeat_runtime, heartbeat_sec.max(1)).await;
                            }));
                        }
                    }
                    message => handle_server_message(runtime.clone(), message).await?,
                }
            }
            Message::Close(frame) => {
                if let Some(frame) = frame {
                    warn!(
                        code = ?frame.code,
                        reason = %frame.reason,
                        "agent websocket closed by server"
                    );
                }
                break;
            }
            _ => {}
        }
    }

    if let Some(heartbeat) = heartbeat {
        heartbeat.abort();
    }
    writer.abort();
    close_all_terminals(runtime.clone()).await;
    cancel_all(runtime).await;
    Ok(())
}

async fn load_or_create_agent_identity(data_dir: &Path) -> anyhow::Result<AgentIdentity> {
    Ok(AgentIdentity {
        id: load_or_create_secret_file(data_dir, "agent.id", "agent").await?,
        token: load_or_create_secret_file(data_dir, "agent.token", "token").await?,
    })
}

async fn load_or_create_secret_file(
    data_dir: &Path,
    filename: &str,
    prefix: &str,
) -> anyhow::Result<String> {
    let token_path = data_dir.join(filename);
    let mut replace_empty_existing = false;
    match fs::read_to_string(&token_path).await {
        Ok(existing) => {
            let existing = existing.trim();
            if !existing.is_empty() {
                return Ok(existing.to_owned());
            }
            replace_empty_existing = true;
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(err).with_context(|| format!("read {}", token_path.display()));
        }
    }

    fs::create_dir_all(data_dir)
        .await
        .with_context(|| format!("create {}", data_dir.display()))?;
    let token = format!("{prefix}_{}", Uuid::new_v4().simple());
    let tmp_path = data_dir.join(format!("{filename}.tmp"));
    fs::write(&tmp_path, format!("{token}\n"))
        .await
        .with_context(|| format!("write {}", tmp_path.display()))?;
    set_private_permissions(&tmp_path).await?;
    if replace_empty_existing {
        remove_empty_token_file(&token_path).await?;
    }
    fs::rename(&tmp_path, &token_path)
        .await
        .with_context(|| format!("install {}", token_path.display()))?;
    set_private_permissions(&token_path).await?;
    info!(path = %token_path.display(), "generated agent token");
    Ok(token)
}

async fn remove_empty_token_file(path: &Path) -> anyhow::Result<()> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("remove {}", path.display())),
    }
}

#[cfg(unix)]
async fn set_private_permissions(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .await
        .with_context(|| format!("set permissions {}", path.display()))
}

#[cfg(not(unix))]
async fn set_private_permissions(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "-".to_owned())
}

fn username() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_owned())
}

fn advertised_ip(config: &AgentConfig) -> String {
    config
        .advertise_ip
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(primary_network_ip)
        .unwrap_or_else(|| "-".to_owned())
}

fn primary_network_ip() -> Option<String> {
    let mut candidates = if_addrs::get_if_addrs()
        .ok()?
        .into_iter()
        .filter_map(|interface| {
            if interface.is_loopback() {
                return None;
            }
            let ip = match interface.ip() {
                std::net::IpAddr::V4(ip) if usable_ipv4(ip) => ip,
                _ => return None,
            };
            let score = physical_interface_score(&interface.name);
            if score == 0 {
                return None;
            }
            Some((score, interface.name, ip))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    candidates
        .into_iter()
        .next()
        .map(|(_, _, ip)| ip.to_string())
}

fn usable_ipv4(ip: Ipv4Addr) -> bool {
    !ip.is_loopback()
        && !ip.is_unspecified()
        && !ip.is_link_local()
        && !ip.is_multicast()
        && !ip.is_broadcast()
}

fn physical_interface_score(name: &str) -> u8 {
    let name = name.to_ascii_lowercase();
    if name == "lo"
        || name.starts_with("lo")
        || name.starts_with("docker")
        || name.starts_with("br-")
        || name.starts_with("veth")
        || name.starts_with("virbr")
        || name.starts_with("vmnet")
        || name.starts_with("tun")
        || name.starts_with("tap")
        || name.starts_with("wg")
        || name.starts_with("tailscale")
        || name.starts_with("zt")
        || name.starts_with("cni")
        || name.starts_with("flannel")
        || name.starts_with("kube")
    {
        return 0;
    }

    if name.starts_with("eth")
        || name.starts_with("eno")
        || name.starts_with("ens")
        || name.starts_with("enp")
        || name.starts_with("em")
        || name == "en0"
        || name == "en1"
        || name.contains("ethernet")
    {
        return 100;
    }

    if name.starts_with("wlan")
        || name.starts_with("wl")
        || name.contains("wi-fi")
        || name.contains("wifi")
        || name.contains("wireless")
    {
        return 90;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_physical_interfaces() {
        assert_eq!(physical_interface_score("eth0"), 100);
        assert_eq!(physical_interface_score("enp3s0"), 100);
        assert_eq!(physical_interface_score("wlan0"), 90);
        assert_eq!(physical_interface_score("Wi-Fi"), 90);
    }

    #[test]
    fn rejects_virtual_interfaces() {
        assert_eq!(physical_interface_score("lo"), 0);
        assert_eq!(physical_interface_score("docker0"), 0);
        assert_eq!(physical_interface_score("veth123"), 0);
        assert_eq!(physical_interface_score("tun0"), 0);
    }

    #[test]
    fn rejects_unsafe_run_work_dir_ids() {
        let work_dir = Path::new("/tmp/buildsvc-agent");
        assert!(run_work_dir(work_dir, "run_123").is_ok());
        assert!(run_work_dir(work_dir, "../run_123").is_err());
        assert!(run_work_dir(work_dir, "run/123").is_err());
        assert!(run_work_dir(work_dir, r"run\123").is_err());
        assert!(run_work_dir(work_dir, "").is_err());
    }

    #[test]
    fn validates_upgrade_package_filename() {
        assert!(validate_upgrade_package_filename(UpgradePackageKind::Deb, "buildsvc.deb").is_ok());
        assert!(validate_upgrade_package_filename(UpgradePackageKind::Rpm, "buildsvc.rpm").is_ok());
        assert!(
            validate_upgrade_package_filename(UpgradePackageKind::Emerge, "overlay.tar.gz").is_ok()
        );
        assert!(
            validate_upgrade_package_filename(UpgradePackageKind::Deb, "../buildsvc.deb").is_err()
        );
        assert!(
            validate_upgrade_package_filename(UpgradePackageKind::Deb, "buildsvc.rpm").is_err()
        );
    }
}

async fn heartbeat_loop(runtime: AgentRuntime, heartbeat_sec: u64) {
    let mut interval = time::interval(Duration::from_secs(heartbeat_sec));
    loop {
        interval.tick().await;
        let runs = runtime.runs.lock().await;
        let snapshots: Vec<AgentRunSnapshot> = runs
            .iter()
            .map(|(run_id, control)| AgentRunSnapshot {
                run_id: run_id.clone(),
                state: control.state.clone(),
            })
            .collect();
        let running = snapshots.len();
        drop(runs);

        if runtime
            .sender
            .send(AgentToServer::Heartbeat {
                running,
                capacity: runtime.config.concurrency.max(1),
                runs: snapshots,
            })
            .is_err()
        {
            break;
        }
    }
}

async fn handle_server_message(
    runtime: AgentRuntime,
    message: ServerToAgent,
) -> anyhow::Result<()> {
    match message {
        ServerToAgent::HelloAccepted { .. } => {}
        ServerToAgent::RunStart {
            run_id,
            build_id,
            source_url,
            archive_format,
            script_timeout_sec,
        } => {
            let effective_timeout = if script_timeout_sec == 0 {
                runtime.config.script_timeout_sec
            } else {
                script_timeout_sec
            };
            start_run(
                runtime,
                run_id,
                build_id,
                source_url,
                archive_format,
                effective_timeout,
            )
            .await?;
        }
        ServerToAgent::RunCancel { run_id, reason } => {
            warn!(%run_id, %reason, "cancel requested");
            cancel_run(runtime, &run_id).await;
        }
        ServerToAgent::RunDelete { run_id } => {
            let result = delete_run_workspace(&runtime, &run_id).await;
            let (success, error) = match result {
                Ok(()) => (true, None),
                Err(err) => (false, Some(err.to_string())),
            };
            let _ = runtime.sender.send(AgentToServer::RunDeleted {
                run_id,
                success,
                error,
            });
        }
        ServerToAgent::TerminalStart {
            session_id,
            rows,
            cols,
        } => {
            if let Err(err) = start_terminal_session(&runtime, session_id.clone(), rows, cols).await
            {
                let _ = runtime.sender.send(AgentToServer::TerminalExit {
                    session_id,
                    exit_code: None,
                    message: Some(err.to_string()),
                });
            }
        }
        ServerToAgent::TerminalInput { session_id, data } => {
            let bytes = BASE64.decode(data)?;
            send_terminal_command(&runtime, &session_id, TerminalCommand::Input(bytes)).await?;
        }
        ServerToAgent::TerminalResize {
            session_id,
            rows,
            cols,
        } => {
            send_terminal_command(
                &runtime,
                &session_id,
                TerminalCommand::Resize {
                    rows: rows.clamp(5, 200),
                    cols: cols.clamp(20, 300),
                },
            )
            .await?;
        }
        ServerToAgent::TerminalClose { session_id } => {
            let _ = send_terminal_command(&runtime, &session_id, TerminalCommand::Close).await;
        }
        ServerToAgent::UpgradeStart {
            upgrade_id,
            package_url,
            package_kind,
            filename,
            sha256,
        } => {
            start_upgrade(
                runtime,
                upgrade_id,
                package_url,
                package_kind,
                filename,
                sha256,
            )
            .await?;
        }
    }
    Ok(())
}

async fn start_run(
    runtime: AgentRuntime,
    run_id: String,
    build_id: String,
    source_url: String,
    archive_format: ArchiveFormat,
    script_timeout_sec: u64,
) -> anyhow::Result<()> {
    let permit = runtime.semaphore.clone().acquire_owned().await?;
    let (cancel_tx, cancel_rx) = oneshot::channel();
    {
        let mut runs = runtime.runs.lock().await;
        if runs.contains_key(&run_id) {
            bail!("run already exists: {run_id}");
        }
        runs.insert(
            run_id.clone(),
            RunControl {
                state: "preparing".to_owned(),
                cancel: cancel_tx,
            },
        );
    }

    let task_runtime = runtime.clone();
    tokio::spawn(async move {
        let _permit = permit;
        let result = execute_run(
            task_runtime.clone(),
            run_id.clone(),
            build_id,
            source_url,
            archive_format,
            script_timeout_sec,
            cancel_rx,
        )
        .await;

        if let Err(err) = result {
            let _ = task_runtime.sender.send(AgentToServer::RunStatus {
                run_id: run_id.clone(),
                state: "failed".to_owned(),
            });
            let encoded = BASE64.encode(format!("agent error: {err}\n"));
            let _ = task_runtime.sender.send(AgentToServer::RunLog {
                run_id: run_id.clone(),
                stream: LogStream::Stderr,
                seq: 0,
                data: encoded,
            });
            let _ = task_runtime.sender.send(AgentToServer::RunFinished {
                run_id: run_id.clone(),
                exit_code: 1,
            });
        }

        task_runtime.runs.lock().await.remove(&run_id);
    });

    Ok(())
}

async fn execute_run(
    runtime: AgentRuntime,
    run_id: String,
    _build_id: String,
    source_url: String,
    archive_format: ArchiveFormat,
    script_timeout_sec: u64,
    cancel_rx: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    set_run_state(&runtime, &run_id, "preparing").await;
    runtime.sender.send(AgentToServer::RunStatus {
        run_id: run_id.clone(),
        state: "preparing".to_owned(),
    })?;

    let run_dir = run_work_dir(&runtime.config.work_dir, &run_id)?;
    let archive_dir = run_dir.join("archive");
    let src_dir = run_dir.join("src");
    fs::create_dir_all(&archive_dir).await?;
    let archive_path = archive_dir.join(format!("source.{}", archive_format.extension()));
    download_source(&runtime, &source_url, &archive_path).await?;

    let archive_path_for_blocking = archive_path.clone();
    let src_dir_for_blocking = src_dir.clone();
    let source_root = tokio::task::spawn_blocking(move || {
        archive::extract_archive(
            &archive_path_for_blocking,
            archive_format,
            &src_dir_for_blocking,
        )
    })
    .await??;

    set_run_state(&runtime, &run_id, "running").await;
    runtime.sender.send(AgentToServer::RunStatus {
        run_id: run_id.clone(),
        state: "running".to_owned(),
    })?;

    let timeout = Duration::from_secs(script_timeout_sec.max(1));
    let exit_code = run_script(
        runtime.clone(),
        run_id.clone(),
        source_root,
        timeout,
        cancel_rx,
    )
    .await?;

    runtime
        .sender
        .send(AgentToServer::RunFinished { run_id, exit_code })?;
    Ok(())
}

async fn download_source(
    runtime: &AgentRuntime,
    source_url: &str,
    destination: &Path,
) -> anyhow::Result<()> {
    let response = runtime
        .client
        .get(source_url)
        .header("x-agent-id", &runtime.identity.id)
        .header("x-agent-token", &runtime.identity.token)
        .send()
        .await?
        .error_for_status()?;
    let bytes = response.bytes().await?;
    fs::write(destination, bytes).await?;
    Ok(())
}

async fn run_script(
    runtime: AgentRuntime,
    run_id: String,
    source_root: PathBuf,
    timeout: Duration,
    mut cancel_rx: oneshot::Receiver<()>,
) -> anyhow::Result<i32> {
    prepare_script(&source_root).await?;
    let mut child = spawn_script(&source_root)?;
    let pid = child.id().context("script process id not available")?;
    let seq = Arc::new(AtomicU64::new(1));

    if let Some(stdout) = child.stdout.take() {
        spawn_log_reader(
            runtime.sender.clone(),
            run_id.clone(),
            LogStream::Stdout,
            seq.clone(),
            stdout,
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_log_reader(
            runtime.sender.clone(),
            run_id.clone(),
            LogStream::Stderr,
            seq.clone(),
            stderr,
        );
    }

    let sleep = time::sleep(timeout);
    tokio::pin!(sleep);
    tokio::select! {
        status = child.wait() => {
            let status = status?;
            Ok(status.code().unwrap_or(1))
        }
        _ = &mut cancel_rx => {
            terminate_child(&mut child, pid, runtime.config.kill_grace_sec).await?;
            Ok(130)
        }
        _ = &mut sleep => {
            terminate_child(&mut child, pid, runtime.config.kill_grace_sec).await?;
            Ok(124)
        }
    }
}

async fn prepare_script(source_root: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = source_root.join("run-build.sh");
        let metadata = fs::metadata(&script)
            .await
            .with_context(|| format!("missing {}", script.display()))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(&script, permissions).await?;
    }

    #[cfg(windows)]
    {
        let script = source_root.join("run-build.bat");
        if fs::metadata(&script).await.is_err() {
            bail!("missing {}", script.display());
        }
    }

    Ok(())
}

fn spawn_script(source_root: &Path) -> anyhow::Result<Child> {
    #[cfg(unix)]
    {
        let mut command = Command::new("./run-build.sh");
        command
            .current_dir(source_root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        command.spawn().context("spawn run-build.sh")
    }

    #[cfg(windows)]
    {
        let mut command = Command::new("cmd.exe");
        command
            .arg("/C")
            .arg("run-build.bat")
            .current_dir(source_root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command.spawn().context("spawn run-build.bat")
    }

    #[cfg(not(any(unix, windows)))]
    {
        bail!("unsupported platform")
    }
}

fn spawn_log_reader<R>(
    sender: mpsc::UnboundedSender<AgentToServer>,
    run_id: String,
    stream: LogStream,
    seq: Arc<AtomicU64>,
    mut reader: R,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buf = vec![0_u8; 8192];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let data = BASE64.encode(&buf[..n]);
                    let next_seq = seq.fetch_add(1, Ordering::Relaxed);
                    if sender
                        .send(AgentToServer::RunLog {
                            run_id: run_id.clone(),
                            stream,
                            seq: next_seq,
                            data,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Err(err) => {
                    warn!(%err, "failed reading script output");
                    break;
                }
            }
        }
    });
}

async fn terminate_child(child: &mut Child, pid: u32, grace_sec: u64) -> anyhow::Result<()> {
    graceful_terminate(pid).await;
    let grace = time::sleep(Duration::from_secs(grace_sec.max(1)));
    tokio::pin!(grace);

    tokio::select! {
        status = child.wait() => {
            let _ = status?;
        }
        _ = &mut grace => {
            force_terminate(pid).await;
            let _ = child.wait().await;
        }
    }
    Ok(())
}

#[cfg(unix)]
async fn graceful_terminate(pid: u32) {
    use nix::{
        sys::signal::{Signal, kill},
        unistd::Pid,
    };
    let _ = kill(Pid::from_raw(-(pid as i32)), Signal::SIGTERM);
}

#[cfg(unix)]
async fn force_terminate(pid: u32) {
    use nix::{
        sys::signal::{Signal, kill},
        unistd::Pid,
    };
    let _ = kill(Pid::from_raw(-(pid as i32)), Signal::SIGKILL);
}

#[cfg(windows)]
async fn graceful_terminate(pid: u32) {
    let _ = Command::new("taskkill")
        .arg("/PID")
        .arg(pid.to_string())
        .arg("/T")
        .status()
        .await;
}

#[cfg(windows)]
async fn force_terminate(pid: u32) {
    let _ = Command::new("taskkill")
        .arg("/PID")
        .arg(pid.to_string())
        .arg("/T")
        .arg("/F")
        .status()
        .await;
}

async fn set_run_state(runtime: &AgentRuntime, run_id: &str, state: &str) {
    let mut runs = runtime.runs.lock().await;
    if let Some(control) = runs.get_mut(run_id) {
        control.state = state.to_owned();
    }
}

async fn cancel_run(runtime: AgentRuntime, run_id: &str) {
    let control = runtime.runs.lock().await.remove(run_id);
    if let Some(control) = control {
        let _ = control.cancel.send(());
    }
}

async fn delete_run_workspace(runtime: &AgentRuntime, run_id: &str) -> anyhow::Result<()> {
    if runtime.runs.lock().await.contains_key(run_id) {
        bail!("run is active; cancel it before deleting");
    }

    let run_dir = run_work_dir(&runtime.config.work_dir, run_id)?;
    match fs::remove_dir_all(&run_dir).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("remove {}", run_dir.display())),
    }
}

async fn start_upgrade(
    runtime: AgentRuntime,
    upgrade_id: String,
    package_url: String,
    package_kind: UpgradePackageKind,
    filename: String,
    sha256: String,
) -> anyhow::Result<()> {
    if let Err(err) = reserve_upgrade(&runtime, &upgrade_id).await {
        send_upgrade_status(&runtime, &upgrade_id, "failed", Some(err.to_string()));
        return Ok(());
    }

    let task_runtime = runtime.clone();
    tokio::spawn(async move {
        let result = execute_upgrade(
            task_runtime.clone(),
            upgrade_id.clone(),
            package_url,
            package_kind,
            filename,
            sha256,
        )
        .await;

        if let Err(err) = result {
            send_upgrade_status(&task_runtime, &upgrade_id, "failed", Some(err.to_string()));
        }
        let mut active = task_runtime.upgrade.lock().await;
        if active.as_deref() == Some(upgrade_id.as_str()) {
            *active = None;
        }
    });

    Ok(())
}

async fn reserve_upgrade(runtime: &AgentRuntime, upgrade_id: &str) -> anyhow::Result<()> {
    if !runtime.config.upgrade_enabled {
        bail!("agent upgrade is disabled");
    }
    if std::env::consts::OS != "linux" {
        bail!("package upgrades are supported only on Linux agents");
    }
    if !runtime.runs.lock().await.is_empty() {
        bail!("agent has active runs");
    }

    let mut active = runtime.upgrade.lock().await;
    if let Some(existing) = active.as_deref() {
        bail!("upgrade already running: {existing}");
    }
    *active = Some(upgrade_id.to_owned());
    Ok(())
}

async fn execute_upgrade(
    runtime: AgentRuntime,
    upgrade_id: String,
    package_url: String,
    package_kind: UpgradePackageKind,
    filename: String,
    sha256: String,
) -> anyhow::Result<()> {
    let filename = validate_upgrade_package_filename(package_kind, &filename)?;
    let upgrade_dir = runtime.config.upgrade_work_dir.join(&upgrade_id);
    match fs::remove_dir_all(&upgrade_dir).await {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err).with_context(|| format!("clear {}", upgrade_dir.display())),
    }
    fs::create_dir_all(&upgrade_dir)
        .await
        .with_context(|| format!("create {}", upgrade_dir.display()))?;

    let package_path = upgrade_dir.join(&filename);
    send_upgrade_status(&runtime, &upgrade_id, "downloading", Some(filename.clone()));
    download_upgrade_package(&runtime, &package_url, &package_path, &sha256).await?;

    send_upgrade_status(
        &runtime,
        &upgrade_id,
        "installing",
        Some(package_kind.to_string()),
    );
    install_upgrade_package(
        &runtime,
        &upgrade_id,
        package_kind,
        &package_path,
        &upgrade_dir,
    )
    .await?;

    send_upgrade_status(&runtime, &upgrade_id, "installed", None);
    run_systemctl_daemon_reload(&runtime, &upgrade_id).await?;
    send_upgrade_status(
        &runtime,
        &upgrade_id,
        "restarting",
        Some("buildsvc".to_owned()),
    );
    request_service_restart().await?;
    send_upgrade_status(&runtime, &upgrade_id, "restart_requested", None);
    Ok(())
}

async fn download_upgrade_package(
    runtime: &AgentRuntime,
    package_url: &str,
    destination: &Path,
    expected_sha256: &str,
) -> anyhow::Result<()> {
    let response = runtime
        .client
        .get(package_url)
        .header("x-agent-id", &runtime.identity.id)
        .header("x-agent-token", &runtime.identity.token)
        .send()
        .await?
        .error_for_status()?;
    let mut stream = response.bytes_stream();
    let mut file = fs::File::create(destination)
        .await
        .with_context(|| format!("create {}", destination.display()))?;
    let mut hasher = Sha256::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;

    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        bail!("sha256 mismatch: expected {expected_sha256}, got {actual}");
    }
    Ok(())
}

async fn install_upgrade_package(
    runtime: &AgentRuntime,
    upgrade_id: &str,
    package_kind: UpgradePackageKind,
    package_path: &Path,
    upgrade_dir: &Path,
) -> anyhow::Result<()> {
    match package_kind {
        UpgradePackageKind::Deb => install_deb(runtime, upgrade_id, package_path).await,
        UpgradePackageKind::Rpm => install_rpm(runtime, upgrade_id, package_path).await,
        UpgradePackageKind::Emerge => {
            install_gentoo_overlay(runtime, upgrade_id, package_path, upgrade_dir).await
        }
    }
}

async fn install_deb(
    runtime: &AgentRuntime,
    upgrade_id: &str,
    package_path: &Path,
) -> anyhow::Result<()> {
    if command_exists("apt-get") {
        let mut command = Command::new("apt-get");
        command
            .env("DEBIAN_FRONTEND", "noninteractive")
            .arg("install")
            .arg("-y")
            .arg("-o")
            .arg("Dpkg::Options::=--force-confold")
            .arg(package_path);
        run_upgrade_command(runtime, upgrade_id, "apt-get install", command).await
    } else if command_exists("dpkg") {
        let mut command = Command::new("dpkg");
        command.arg("--force-confold").arg("-i").arg(package_path);
        run_upgrade_command(runtime, upgrade_id, "dpkg -i", command).await
    } else {
        bail!("neither apt-get nor dpkg was found")
    }
}

async fn install_rpm(
    runtime: &AgentRuntime,
    upgrade_id: &str,
    package_path: &Path,
) -> anyhow::Result<()> {
    if command_exists("dnf") {
        let mut command = Command::new("dnf");
        command.arg("install").arg("-y").arg(package_path);
        run_upgrade_command(runtime, upgrade_id, "dnf install", command).await
    } else if command_exists("yum") {
        let mut command = Command::new("yum");
        command.arg("install").arg("-y").arg(package_path);
        run_upgrade_command(runtime, upgrade_id, "yum install", command).await
    } else if command_exists("rpm") {
        let mut command = Command::new("rpm");
        command.arg("-Uvh").arg("--replacepkgs").arg(package_path);
        run_upgrade_command(runtime, upgrade_id, "rpm -Uvh", command).await
    } else {
        bail!("dnf, yum, and rpm were not found")
    }
}

async fn install_gentoo_overlay(
    runtime: &AgentRuntime,
    upgrade_id: &str,
    package_path: &Path,
    upgrade_dir: &Path,
) -> anyhow::Result<()> {
    if !command_exists("emerge") {
        bail!("emerge was not found");
    }

    let extract_dir = upgrade_dir.join("gentoo");
    let overlay = extract_gentoo_overlay(package_path, &extract_dir).await?;
    let mut command = Command::new("emerge");
    command
        .env("PORTDIR_OVERLAY", &overlay)
        .arg("app-admin/buildsvc");
    run_upgrade_command(runtime, upgrade_id, "emerge app-admin/buildsvc", command).await
}

async fn extract_gentoo_overlay(
    package_path: &Path,
    destination: &Path,
) -> anyhow::Result<PathBuf> {
    let package_path = package_path.to_path_buf();
    let destination = destination.to_path_buf();
    tokio::task::spawn_blocking(move || {
        if destination.exists() {
            std::fs::remove_dir_all(&destination)
                .with_context(|| format!("clear {}", destination.display()))?;
        }
        std::fs::create_dir_all(&destination)
            .with_context(|| format!("create {}", destination.display()))?;
        let file = std::fs::File::open(&package_path)
            .with_context(|| format!("open {}", package_path.display()))?;
        let decoder = GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(&destination)
            .with_context(|| format!("extract {}", package_path.display()))?;
        let overlay = destination.join("gentoo-overlay");
        if !overlay.join("profiles").join("repo_name").is_file() {
            bail!("gentoo overlay tarball does not contain gentoo-overlay/profiles/repo_name");
        }
        Ok(overlay)
    })
    .await?
}

async fn run_systemctl_daemon_reload(
    runtime: &AgentRuntime,
    upgrade_id: &str,
) -> anyhow::Result<()> {
    if !command_exists("systemctl") {
        bail!("systemctl was not found");
    }
    let mut command = Command::new("systemctl");
    command.arg("daemon-reload");
    run_upgrade_command(runtime, upgrade_id, "systemctl daemon-reload", command).await
}

async fn request_service_restart() -> anyhow::Result<()> {
    let mut command = Command::new("systemctl");
    command
        .arg("restart")
        .arg("buildsvc")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
        .spawn()
        .context("spawn systemctl restart buildsvc")?;
    Ok(())
}

async fn run_upgrade_command(
    runtime: &AgentRuntime,
    upgrade_id: &str,
    label: &str,
    mut command: Command,
) -> anyhow::Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().with_context(|| format!("spawn {label}"))?;
    let seq = Arc::new(AtomicU64::new(1));

    if let Some(stdout) = child.stdout.take() {
        spawn_upgrade_log_reader(
            runtime.sender.clone(),
            upgrade_id.to_owned(),
            LogStream::Stdout,
            seq.clone(),
            stdout,
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_upgrade_log_reader(
            runtime.sender.clone(),
            upgrade_id.to_owned(),
            LogStream::Stderr,
            seq,
            stderr,
        );
    }

    let status = child.wait().await?;
    if !status.success() {
        bail!("{label} exited with {}", status.code().unwrap_or(1));
    }
    Ok(())
}

fn spawn_upgrade_log_reader<R>(
    sender: mpsc::UnboundedSender<AgentToServer>,
    upgrade_id: String,
    stream: LogStream,
    seq: Arc<AtomicU64>,
    mut reader: R,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buf = vec![0_u8; 8192];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let data = BASE64.encode(&buf[..n]);
                    let next_seq = seq.fetch_add(1, Ordering::Relaxed);
                    if sender
                        .send(AgentToServer::UpgradeLog {
                            upgrade_id: upgrade_id.clone(),
                            stream,
                            seq: next_seq,
                            data,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Err(err) => {
                    let _ = sender.send(AgentToServer::UpgradeStatus {
                        upgrade_id: upgrade_id.clone(),
                        state: "failed".to_owned(),
                        message: Some(format!("failed reading upgrade output: {err}")),
                    });
                    break;
                }
            }
        }
    });
}

fn send_upgrade_status(
    runtime: &AgentRuntime,
    upgrade_id: &str,
    state: &str,
    message: Option<String>,
) {
    let _ = runtime.sender.send(AgentToServer::UpgradeStatus {
        upgrade_id: upgrade_id.to_owned(),
        state: state.to_owned(),
        message,
    });
}

fn validate_upgrade_package_filename(
    kind: UpgradePackageKind,
    filename: &str,
) -> anyhow::Result<String> {
    if filename.is_empty()
        || filename == "."
        || filename == ".."
        || filename.contains('/')
        || filename.contains('\\')
    {
        bail!("invalid upgrade package filename");
    }

    let lower = filename.to_ascii_lowercase();
    let valid = match kind {
        UpgradePackageKind::Deb => lower.ends_with(".deb"),
        UpgradePackageKind::Rpm => lower.ends_with(".rpm"),
        UpgradePackageKind::Emerge => lower.ends_with(".tar.gz") || lower.ends_with(".tgz"),
    };
    if !valid {
        bail!("upgrade package filename does not match kind {kind}");
    }
    Ok(filename.to_owned())
}

fn command_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let path = dir.join(name);
                path.is_file()
            })
        })
        .unwrap_or(false)
}

fn run_work_dir(work_dir: &Path, run_id: &str) -> anyhow::Result<PathBuf> {
    if run_id.is_empty()
        || run_id == "."
        || run_id == ".."
        || run_id.contains('/')
        || run_id.contains('\\')
    {
        bail!("invalid run id");
    }
    Ok(work_dir.join("runs").join(run_id))
}

async fn start_terminal_session(
    runtime: &AgentRuntime,
    session_id: String,
    rows: u16,
    cols: u16,
) -> anyhow::Result<()> {
    if !runtime.config.terminal_enabled {
        bail!("terminal is disabled");
    }
    fs::create_dir_all(&runtime.config.terminal_work_dir)
        .await
        .with_context(|| format!("create {}", runtime.config.terminal_work_dir.display()))?;

    let mut sessions = runtime.terminal_sessions.lock().await;
    if sessions.contains_key(&session_id) {
        bail!("terminal session already exists: {session_id}");
    }
    if sessions.len() >= runtime.config.terminal_max_sessions {
        bail!("terminal session limit reached");
    }

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: rows.clamp(5, 200),
        cols: cols.clamp(20, 300),
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut command = CommandBuilder::new(terminal_shell(&runtime.config));
    command.cwd(runtime.config.terminal_work_dir.as_os_str());
    command.env("TERM", "xterm-256color");

    let mut child = pair.slave.spawn_command(command)?;
    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;
    let mut killer = child.clone_killer();
    let master = pair.master;
    let (tx, mut rx) = mpsc::unbounded_channel::<TerminalCommand>();

    sessions.insert(session_id.clone(), TerminalControl { sender: tx });
    drop(sessions);

    let _ = runtime.sender.send(AgentToServer::TerminalStarted {
        session_id: session_id.clone(),
    });

    let output_sender = runtime.sender.clone();
    let output_session = session_id.clone();
    std::thread::spawn(move || {
        let mut buf = vec![0_u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = output_sender.send(AgentToServer::TerminalOutput {
                        session_id: output_session.clone(),
                        data: BASE64.encode(&buf[..n]),
                    });
                }
                Err(err) => {
                    let _ = output_sender.send(AgentToServer::TerminalExit {
                        session_id: output_session.clone(),
                        exit_code: None,
                        message: Some(format!("terminal read failed: {err}")),
                    });
                    break;
                }
            }
        }
    });

    std::thread::spawn(move || {
        while let Some(command) = rx.blocking_recv() {
            match command {
                TerminalCommand::Input(bytes) => {
                    if writer.write_all(&bytes).is_err() || writer.flush().is_err() {
                        break;
                    }
                }
                TerminalCommand::Resize { rows, cols } => {
                    let _ = master.resize(PtySize {
                        rows: rows.clamp(5, 200),
                        cols: cols.clamp(20, 300),
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
                TerminalCommand::Close => {
                    let _ = killer.kill();
                    break;
                }
            }
        }
    });

    let wait_sender = runtime.sender.clone();
    let wait_sessions = runtime.terminal_sessions.clone();
    let wait_session = session_id.clone();
    std::thread::spawn(move || {
        let result = child.wait();
        wait_sessions.blocking_lock().remove(&wait_session);
        match result {
            Ok(status) => {
                let _ = wait_sender.send(AgentToServer::TerminalExit {
                    session_id: wait_session,
                    exit_code: Some(status.exit_code() as i32),
                    message: status.signal().map(|signal| signal.to_owned()),
                });
            }
            Err(err) => {
                let _ = wait_sender.send(AgentToServer::TerminalExit {
                    session_id: wait_session,
                    exit_code: None,
                    message: Some(format!("terminal wait failed: {err}")),
                });
            }
        }
    });

    Ok(())
}

async fn send_terminal_command(
    runtime: &AgentRuntime,
    session_id: &str,
    command: TerminalCommand,
) -> anyhow::Result<()> {
    let sessions = runtime.terminal_sessions.lock().await;
    let session = sessions
        .get(session_id)
        .with_context(|| format!("terminal session {session_id} not found"))?;
    session.sender.send(command)?;
    Ok(())
}

fn terminal_shell(config: &AgentConfig) -> String {
    if let Some(shell) = config
        .terminal_shell
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return shell.to_owned();
    }

    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_owned())
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned())
    }
}

async fn cancel_all(runtime: AgentRuntime) {
    let mut runs = runtime.runs.lock().await;
    let controls: Vec<RunControl> = runs.drain().map(|(_, control)| control).collect();
    drop(runs);
    for control in controls {
        let _ = control.cancel.send(());
    }
}

async fn close_all_terminals(runtime: AgentRuntime) {
    let mut sessions = runtime.terminal_sessions.lock().await;
    let controls = sessions
        .drain()
        .map(|(_, control)| control)
        .collect::<Vec<_>>();
    drop(sessions);
    for control in controls {
        let _ = control.sender.send(TerminalCommand::Close);
    }
}

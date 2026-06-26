use std::{
    collections::HashMap,
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
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use tokio::{
    fs,
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
    sync::{Mutex, Semaphore, mpsc, oneshot},
    time,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

use crate::{
    archive,
    config::{AgentConfig, CoreConfig},
    protocol::{AgentRunSnapshot, AgentToServer, ArchiveFormat, LogStream, ServerToAgent},
};

#[derive(Clone)]
struct AgentRuntime {
    config: AgentConfig,
    client: Client,
    sender: mpsc::UnboundedSender<AgentToServer>,
    runs: Arc<Mutex<HashMap<String, RunControl>>>,
    semaphore: Arc<Semaphore>,
}

struct RunControl {
    state: String,
    cancel: oneshot::Sender<()>,
}

pub async fn run(core: CoreConfig, config: AgentConfig) -> anyhow::Result<()> {
    fs::create_dir_all(&config.work_dir)
        .await
        .with_context(|| format!("create {}", config.work_dir.display()))?;
    fs::create_dir_all(core.data_dir.join("tmp")).await.ok();

    loop {
        match run_once(config.clone()).await {
            Ok(()) => warn!("agent websocket closed"),
            Err(err) => warn!(%err, "agent websocket error"),
        }
        time::sleep(Duration::from_secs(3)).await;
    }
}

async fn run_once(config: AgentConfig) -> anyhow::Result<()> {
    let (ws, _) = connect_async(&config.server_url)
        .await
        .with_context(|| format!("connect {}", config.server_url))?;
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<AgentToServer>();
    let runtime = AgentRuntime {
        config: config.clone(),
        client: Client::new(),
        sender: out_tx.clone(),
        runs: Arc::new(Mutex::new(HashMap::new())),
        semaphore: Arc::new(Semaphore::new(config.concurrency.max(1))),
    };

    out_tx.send(AgentToServer::Hello {
        name: config.name.clone(),
        token: config.token.clone(),
        computer_name: computer_name(&config.name),
        username: username(),
        ip: advertised_ip(&config),
        labels: config.labels.clone(),
        platform: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        concurrency: config.concurrency.max(1),
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
            Message::Close(_) => break,
            _ => {}
        }
    }

    if let Some(heartbeat) = heartbeat {
        heartbeat.abort();
    }
    writer.abort();
    cancel_all(runtime).await;
    Ok(())
}

fn computer_name(fallback: &str) -> String {
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
        .unwrap_or_else(|| fallback.to_owned())
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

    let run_dir = runtime.config.work_dir.join("runs").join(&run_id);
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
        .header("x-agent-name", &runtime.config.name)
        .header("x-agent-token", &runtime.config.token)
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

async fn cancel_all(runtime: AgentRuntime) {
    let mut runs = runtime.runs.lock().await;
    let controls: Vec<RunControl> = runs.drain().map(|(_, control)| control).collect();
    drop(runs);
    for control in controls {
        let _ = control.cancel.send(());
    }
}

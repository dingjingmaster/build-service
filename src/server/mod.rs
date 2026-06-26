use std::{
    collections::{BTreeSet, HashMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, bail};
use axum::{
    Router,
    body::Body,
    extract::{
        DefaultBodyLimit, Multipart, Path, State,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    fs,
    io::AsyncWriteExt,
    net::TcpListener,
    sync::{broadcast, mpsc, oneshot},
    time,
};
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

use crate::{
    config::{CoreConfig, ServerConfig},
    ids,
    protocol::{
        AgentToServer, AgentView, ArchiveFormat, ServerToAgent, UiMessage, UiState,
        UpgradePackageKind,
    },
    storage::{NewRun, Storage, now_ts},
};

mod ui;

const RUN_DELETE_TIMEOUT_SEC: u64 = 30;

#[derive(Clone)]
struct AppState {
    server: ServerConfig,
    storage: Storage,
    runtime: Arc<Mutex<RuntimeState>>,
    ui_tx: broadcast::Sender<UiMessage>,
}

struct RuntimeState {
    agents: HashMap<String, AgentRuntime>,
    agent_tokens: HashMap<String, String>,
    agent_metadata: HashMap<String, AgentMetadata>,
    pending_deletes: HashMap<String, PendingDelete>,
    terminal_sessions: HashMap<String, TerminalSession>,
    upgrades: HashMap<String, UpgradePackage>,
}

struct AgentMetadata {
    computer_name: Option<String>,
    username: Option<String>,
    ip: Option<String>,
    platform: Option<String>,
    arch: Option<String>,
    version: Option<String>,
}

struct AgentRuntime {
    id: String,
    computer_name: String,
    username: String,
    ip: String,
    platform: String,
    arch: String,
    version: String,
    concurrency: usize,
    terminal_enabled: bool,
    upgrade_enabled: bool,
    upgrade_status: Option<String>,
    running: HashSet<String>,
    last_seen: i64,
    sender: mpsc::UnboundedSender<ServerToAgent>,
}

struct PendingDelete {
    agent_id: String,
    sender: oneshot::Sender<Result<(), String>>,
}

struct TerminalSession {
    agent_id: String,
    sender: mpsc::UnboundedSender<TerminalUiMessage>,
}

#[derive(Clone)]
struct UpgradePackage {
    filename: String,
    path: PathBuf,
}

#[derive(Debug, Serialize)]
struct UpgradeResponse {
    state: UiState,
    upgrade_id: String,
    sent: Vec<String>,
    failed: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TerminalUiMessage {
    Open {
        session_id: String,
        agent_id: String,
    },
    Output {
        data: String,
    },
    Exit {
        exit_code: Option<i32>,
        message: Option<String>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TerminalBrowserMessage {
    Input { data: String },
    Resize { rows: u16, cols: u16 },
    Close,
}

pub async fn run(core: CoreConfig, server: ServerConfig) -> anyhow::Result<()> {
    let storage = Storage::open(core.data_dir.clone(), server.db_path.clone())?;
    let lost_on_start = storage.mark_active_runs_lost()?;
    if !lost_on_start.is_empty() {
        warn!(
            count = lost_on_start.len(),
            "marked active runs lost on server startup"
        );
    }
    storage.cleanup_old_logs(server.log_retention_days)?;

    let (ui_tx, _) = broadcast::channel(256);
    let state = AppState {
        server: server.clone(),
        storage,
        runtime: Arc::new(Mutex::new(RuntimeState {
            agents: HashMap::new(),
            agent_tokens: HashMap::new(),
            agent_metadata: HashMap::new(),
            pending_deletes: HashMap::new(),
            terminal_sessions: HashMap::new(),
            upgrades: HashMap::new(),
        })),
        ui_tx,
    };

    spawn_log_cleanup(state.clone());
    spawn_agent_timeout_monitor(state.clone());

    let max_upload = server.max_upload_size_mb.saturating_mul(1024 * 1024) as usize;
    let app = Router::new()
        .route("/", get(index))
        .route("/api/state", get(api_state))
        .route("/api/builds", post(upload_build))
        .route("/api/builds/{build_id}", delete(delete_build))
        .route("/api/upgrades", post(upload_upgrade))
        .route("/api/upgrades/{upgrade_id}/package", get(download_upgrade))
        .route("/api/runs/{run_id}/source", get(download_source))
        .route("/api/runs/{run_id}/log", get(read_log))
        .route("/api/runs/{run_id}", delete(delete_run))
        .route("/api/runs/{run_id}/rerun", post(rerun))
        .route("/api/runs/{run_id}/cancel", post(cancel))
        .route("/api/agents/{agent_id}", delete(delete_agent))
        .route("/api/agents/{agent_id}/terminal/ws", get(terminal_ws))
        .route("/api/agent/ws", get(agent_ws))
        .route("/api/ui/ws", get(ui_ws))
        .layer(DefaultBodyLimit::max(max_upload))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let addr: SocketAddr = server
        .listen
        .parse()
        .with_context(|| format!("parse listen address {}", server.listen))?;
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "buildsvc server listening");
    info!(
        agent_timeout_sec = server.agent_offline_after_sec,
        cancel_grace_sec = server.kill_grace_sec,
        "server runtime policy loaded"
    );
    axum::serve(listener, app).await?;
    Ok(())
}

fn spawn_log_cleanup(state: AppState) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            if let Err(err) = state
                .storage
                .cleanup_old_logs(state.server.log_retention_days)
            {
                warn!(%err, "log cleanup failed");
            }
        }
    });
}

fn spawn_agent_timeout_monitor(state: AppState) {
    tokio::spawn(async move {
        let check_sec = state.server.agent_heartbeat_sec.clamp(1, 5);
        let mut interval = time::interval(Duration::from_secs(check_sec));
        loop {
            interval.tick().await;
            let cutoff = now_ts() - state.server.agent_offline_after_sec as i64;
            let stale_agents = {
                let runtime = state.runtime.lock();
                let Ok(runtime) = runtime else {
                    continue;
                };
                runtime
                    .agents
                    .values()
                    .filter(|agent| agent.last_seen < cutoff)
                    .map(|agent| agent.id.clone())
                    .collect::<Vec<_>>()
            };

            for agent_id in stale_agents {
                if let Err(err) = disconnect_agent(&state, &agent_id) {
                    warn!(agent = %agent_id, %err, "failed to disconnect stale agent");
                }
            }
        }
    });
}

async fn index() -> Html<String> {
    Html(ui::index_html())
}

async fn api_state(State(state): State<AppState>) -> Result<JsonResponse<UiState>, ApiError> {
    Ok(JsonResponse(state.ui_state()?))
}

async fn read_log(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Response, ApiError> {
    let log = state.storage.read_log(&run_id)?;
    Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], log).into_response())
}

async fn upload_build(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<JsonResponse<UiState>, ApiError> {
    let build_id = ids::build_id();
    let build_dir = state.storage.sources_dir().join(&build_id);
    fs::create_dir_all(&build_dir).await?;

    let mut source_name = None;
    let mut archive_format = None;
    let mut archive_path = None;
    let mut uploaded_size = 0_u64;
    let mut target_agent_ids = String::new();

    while let Some(mut field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_owned();
        match name.as_str() {
            "source" => {
                let filename = field
                    .file_name()
                    .map(ToOwned::to_owned)
                    .context("missing source filename")?;
                let format = ArchiveFormat::from_filename(&filename)
                    .with_context(|| format!("unsupported archive format for {filename}"))?;
                let path = build_dir.join(format!("source.{}", format.extension()));
                let mut output = fs::File::create(&path).await?;
                while let Some(chunk) = field.chunk().await? {
                    uploaded_size += chunk.len() as u64;
                    output.write_all(&chunk).await?;
                }
                output.flush().await?;
                source_name = Some(filename);
                archive_format = Some(format);
                archive_path = Some(path);
            }
            "target_agents" => {
                target_agent_ids = field.text().await.unwrap_or_default();
            }
            _ => {}
        }
    }

    let source_name = source_name.context("missing source file")?;
    if uploaded_size == 0 {
        return Err(anyhow::anyhow!("source file is empty").into());
    }
    let archive_format = archive_format.context("missing archive format")?;
    let archive_path = archive_path.context("missing archive path")?;
    let selected = state.resolve_targets(&target_agent_ids)?;
    if selected.is_empty() {
        return Err(anyhow::anyhow!("no target agents selected").into());
    }

    let runs: Vec<NewRun> = selected
        .into_iter()
        .map(|agent_id| NewRun {
            id: ids::run_id(),
            agent_id,
            labels: Vec::new(),
            script_timeout_sec: state.server.script_timeout_sec,
        })
        .collect();
    state.storage.create_build(
        &build_id,
        &source_name,
        archive_format,
        &archive_path,
        &runs,
    )?;
    state.dispatch_queued()?;
    state.broadcast_state();
    Ok(JsonResponse(state.ui_state()?))
}

async fn upload_upgrade(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<JsonResponse<UpgradeResponse>, ApiError> {
    if !state.server.upgrade_enabled {
        return Err(anyhow::anyhow!("server upgrade is disabled").into());
    }

    let upgrade_id = ids::upgrade_id();
    let upgrade_dir = state.storage.upgrades_dir().join(&upgrade_id);
    fs::create_dir_all(&upgrade_dir).await?;

    let mut package_kind = String::new();
    let mut target_agents = String::new();
    let mut package_filename = None;
    let mut package_path = None;
    let mut uploaded_size = 0_u64;
    let mut hasher = Sha256::new();

    while let Some(mut field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_owned();
        match name.as_str() {
            "package_kind" => {
                package_kind = field.text().await.unwrap_or_default();
            }
            "target_agents" => {
                target_agents = field.text().await.unwrap_or_default();
            }
            "package" => {
                let filename = field
                    .file_name()
                    .map(ToOwned::to_owned)
                    .context("missing package filename")?;
                let safe_filename = sanitize_filename(&filename);
                let path = upgrade_dir.join(&safe_filename);
                let mut output = fs::File::create(&path).await?;
                while let Some(chunk) = field.chunk().await? {
                    uploaded_size += chunk.len() as u64;
                    hasher.update(&chunk);
                    output.write_all(&chunk).await?;
                }
                output.flush().await?;
                package_filename = Some(safe_filename);
                package_path = Some(path);
            }
            _ => {}
        }
    }

    if uploaded_size == 0 {
        return Err(anyhow::anyhow!("upgrade package is empty").into());
    }
    let kind: UpgradePackageKind = package_kind.parse()?;
    let filename = package_filename.context("missing upgrade package")?;
    validate_upgrade_filename(kind, &filename)?;
    let path = package_path.context("missing upgrade package path")?;
    let targets = split_csv(&target_agents);
    if targets.is_empty() {
        return Err(anyhow::anyhow!("no target agents selected").into());
    }
    let sha256 = format!("{:x}", hasher.finalize());
    let package_url = format!(
        "{}/api/upgrades/{}/package",
        state.server.public_url.trim_end_matches('/'),
        upgrade_id
    );

    let mut sent = Vec::new();
    let mut failed = Vec::new();
    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        runtime.upgrades.insert(
            upgrade_id.clone(),
            UpgradePackage {
                filename: filename.clone(),
                path,
            },
        );

        for agent_id in targets {
            let Some(agent) = runtime.agents.get_mut(&agent_id) else {
                failed.push(format!("{agent_id}: offline"));
                continue;
            };
            if !agent.upgrade_enabled {
                failed.push(format!("{agent_id}: upgrade disabled"));
                continue;
            }
            if agent.platform != "linux" {
                failed.push(format!("{agent_id}: package upgrades are Linux-only"));
                continue;
            }
            if !agent.running.is_empty() {
                failed.push(format!("{agent_id}: active runs must finish first"));
                continue;
            }
            match agent.sender.send(ServerToAgent::UpgradeStart {
                upgrade_id: upgrade_id.clone(),
                package_url: package_url.clone(),
                package_kind: kind,
                filename: filename.clone(),
                sha256: sha256.clone(),
            }) {
                Ok(()) => {
                    agent.upgrade_status = Some(format!("queued {upgrade_id}"));
                    sent.push(agent_id);
                }
                Err(err) => failed.push(format!("{agent_id}: {err}")),
            }
        }
    }

    if sent.is_empty() {
        return Err(anyhow::anyhow!(
            "no agent accepted upgrade{}",
            if failed.is_empty() {
                String::new()
            } else {
                format!(": {}", failed.join("; "))
            }
        )
        .into());
    }

    state.broadcast_state();
    Ok(JsonResponse(UpgradeResponse {
        state: state.ui_state()?,
        upgrade_id,
        sent,
        failed,
    }))
}

async fn download_source(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let agent_id = header_value(&headers, "x-agent-id").context("missing x-agent-id")?;
    let token = header_value(&headers, "x-agent-token").context("missing x-agent-token")?;
    state.verify_agent_token(agent_id, token)?;

    let run = state.storage.get_run(&run_id)?.context("run not found")?;
    if run.agent_id != agent_id {
        return Err(anyhow::anyhow!("run is not assigned to this agent").into());
    }
    let bytes = fs::read(&run.source_path).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"source.archive\"",
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}

async fn download_upgrade(
    State(state): State<AppState>,
    Path(upgrade_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let agent_id = header_value(&headers, "x-agent-id").context("missing x-agent-id")?;
    let token = header_value(&headers, "x-agent-token").context("missing x-agent-token")?;
    state.verify_agent_token(agent_id, token)?;

    let package = {
        let runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        runtime
            .upgrades
            .get(&upgrade_id)
            .cloned()
            .context("upgrade package not found")?
    };
    let bytes = fs::read(&package.path).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!(
                    "attachment; filename=\"{}\"",
                    package.filename.replace(['"', '\\'], "_")
                ),
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}

async fn delete_build(
    State(state): State<AppState>,
    Path(build_id): Path<String>,
) -> Result<JsonResponse<UiState>, ApiError> {
    state.storage.delete_build(&build_id)?;
    state.broadcast_state();
    Ok(JsonResponse(state.ui_state()?))
}

async fn rerun(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<JsonResponse<UiState>, ApiError> {
    let old = state.storage.get_run(&run_id)?.context("run not found")?;
    if !matches!(
        old.status.as_str(),
        "failed" | "lost" | "canceled" | "success"
    ) {
        return Err(anyhow::anyhow!("run is not finished").into());
    }
    let new_id = ids::run_id();
    state.storage.create_rerun(&old, &new_id)?;
    state.dispatch_queued()?;
    state.broadcast_state();
    Ok(JsonResponse(state.ui_state()?))
}

async fn cancel(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<JsonResponse<UiState>, ApiError> {
    let run = state.storage.get_run(&run_id)?.context("run not found")?;
    let mut sent = false;
    {
        let runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        if let Some(agent) = runtime.agents.get(&run.agent_id) {
            sent = agent
                .sender
                .send(ServerToAgent::RunCancel {
                    run_id: run_id.clone(),
                    reason: "user_requested".to_owned(),
                })
                .is_ok();
        }
    }
    if !sent {
        state.storage.cancel_run(&run_id)?;
    }
    state.broadcast_state();
    Ok(JsonResponse(state.ui_state()?))
}

async fn delete_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<JsonResponse<UiState>, ApiError> {
    let run = state.storage.get_run(&run_id)?.context("run not found")?;
    if !is_terminal_run_status(&run.status) {
        return Err(anyhow::anyhow!("run is not finished; cancel it before deleting").into());
    }

    let (ack_tx, ack_rx) = oneshot::channel::<Result<(), String>>();
    let sender = {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        if runtime.pending_deletes.contains_key(&run_id) {
            return Err(anyhow::anyhow!("run deletion is already in progress").into());
        }
        let sender = runtime
            .agents
            .get(&run.agent_id)
            .with_context(|| format!("agent {} is offline", run.agent_id))?
            .sender
            .clone();
        runtime.pending_deletes.insert(
            run_id.clone(),
            PendingDelete {
                agent_id: run.agent_id.clone(),
                sender: ack_tx,
            },
        );
        sender
    };

    if let Err(err) = sender.send(ServerToAgent::RunDelete {
        run_id: run_id.clone(),
    }) {
        state.remove_pending_delete(&run_id);
        return Err(anyhow::anyhow!("failed to send delete request to agent: {err}").into());
    }

    match time::timeout(Duration::from_secs(RUN_DELETE_TIMEOUT_SEC), ack_rx).await {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(err))) => return Err(anyhow::anyhow!(err).into()),
        Ok(Err(_)) => return Err(anyhow::anyhow!("agent delete response was dropped").into()),
        Err(_) => {
            state.remove_pending_delete(&run_id);
            return Err(
                anyhow::anyhow!("timed out waiting for agent to delete run workspace").into(),
            );
        }
    }

    state.storage.delete_run(&run_id)?;
    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        if let Some(agent) = runtime.agents.get_mut(&run.agent_id) {
            agent.running.remove(&run_id);
        }
    }
    state.broadcast_state();
    Ok(JsonResponse(state.ui_state()?))
}

async fn delete_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<JsonResponse<UiState>, ApiError> {
    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        if runtime.agents.contains_key(&agent_id) {
            return Err(anyhow::anyhow!("agent {agent_id} is online").into());
        }
        if !runtime.agent_metadata.contains_key(&agent_id)
            && !runtime.agent_tokens.contains_key(&agent_id)
        {
            return Err(anyhow::anyhow!("agent {agent_id} not found").into());
        }
        runtime.agent_tokens.remove(&agent_id);
        runtime.agent_metadata.remove(&agent_id);
    }
    state.broadcast_state();
    Ok(JsonResponse(state.ui_state()?))
}

async fn terminal_ws(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal_ws(state, agent_id, socket))
}

async fn handle_terminal_ws(state: AppState, agent_id: String, socket: WebSocket) {
    if let Err(err) = terminal_ws_inner(state, agent_id, socket).await {
        warn!(%err, "terminal websocket closed with error");
    }
}

async fn terminal_ws_inner(
    state: AppState,
    agent_id: String,
    socket: WebSocket,
) -> anyhow::Result<()> {
    let (mut ws_tx, mut ws_rx) = socket.split();
    if !state.server.terminal_enabled {
        send_terminal_ws_message(
            &mut ws_tx,
            &TerminalUiMessage::Error {
                message: "server terminal is disabled".to_owned(),
            },
        )
        .await;
        return Ok(());
    }

    let session_id = ids::terminal_session_id();
    let (terminal_tx, mut terminal_rx) = mpsc::unbounded_channel::<TerminalUiMessage>();
    let agent_sender = {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        let agent = runtime
            .agents
            .get(&agent_id)
            .with_context(|| format!("agent {agent_id} is offline"))?;
        if !agent.terminal_enabled {
            bail!("agent {agent_id} terminal is disabled");
        }
        let sender = agent.sender.clone();
        runtime.terminal_sessions.insert(
            session_id.clone(),
            TerminalSession {
                agent_id: agent_id.clone(),
                sender: terminal_tx.clone(),
            },
        );
        sender
    };

    if let Err(err) = agent_sender.send(ServerToAgent::TerminalStart {
        session_id: session_id.clone(),
        rows: 32,
        cols: 120,
    }) {
        state.remove_terminal_session(&session_id);
        bail!("failed to start terminal on agent: {err}");
    }

    let _ = terminal_tx.send(TerminalUiMessage::Open {
        session_id: session_id.clone(),
        agent_id: agent_id.clone(),
    });

    let writer = tokio::spawn(async move {
        while let Some(message) = terminal_rx.recv().await {
            let Ok(text) = serde_json::to_string(&message) else {
                continue;
            };
            if ws_tx.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = ws_rx.next().await {
        let message = message?;
        match message {
            Message::Text(text) => {
                let browser_message: TerminalBrowserMessage = serde_json::from_str(&text)?;
                match browser_message {
                    TerminalBrowserMessage::Input { data } => {
                        state.send_terminal_to_agent(
                            &session_id,
                            ServerToAgent::TerminalInput {
                                session_id: session_id.clone(),
                                data,
                            },
                        )?;
                    }
                    TerminalBrowserMessage::Resize { rows, cols } => {
                        state.send_terminal_to_agent(
                            &session_id,
                            ServerToAgent::TerminalResize {
                                session_id: session_id.clone(),
                                rows: rows.clamp(5, 200),
                                cols: cols.clamp(20, 300),
                            },
                        )?;
                    }
                    TerminalBrowserMessage::Close => break,
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    state.close_terminal_session(&session_id);
    writer.abort();
    Ok(())
}

async fn send_terminal_ws_message(
    ws_tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: &TerminalUiMessage,
) {
    if let Ok(text) = serde_json::to_string(message) {
        let _ = ws_tx.send(Message::Text(text.into())).await;
    }
}

async fn agent_ws(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_agent_ws(state, socket))
}

async fn handle_agent_ws(state: AppState, socket: WebSocket) {
    if let Err(err) = agent_ws_inner(state, socket).await {
        warn!(%err, "agent websocket closed with error");
    }
}

async fn agent_ws_inner(state: AppState, socket: WebSocket) -> anyhow::Result<()> {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let Some(Ok(Message::Text(first))) = ws_rx.next().await else {
        bail!("agent did not send hello");
    };
    let hello: AgentToServer = serde_json::from_str(&first)?;
    let AgentToServer::Hello {
        agent_id,
        token,
        computer_name,
        username,
        ip,
        platform,
        arch,
        concurrency,
        terminal_enabled,
        upgrade_enabled,
        version,
    } = hello
    else {
        bail!("first agent message must be hello");
    };
    if let Err(err) = state.accept_agent_token(&agent_id, &token) {
        let reason = truncate_close_reason(&err.to_string());
        let _ = ws_tx
            .send(Message::Close(Some(CloseFrame {
                code: 1008,
                reason: reason.into(),
            })))
            .await;
        return Err(err);
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<ServerToAgent>();
    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        runtime.agents.insert(
            agent_id.clone(),
            AgentRuntime {
                id: agent_id.clone(),
                computer_name: computer_name.clone(),
                username: username.clone(),
                ip: ip.clone(),
                platform: platform.clone(),
                arch: arch.clone(),
                version: version.clone(),
                concurrency: concurrency.max(1),
                terminal_enabled,
                upgrade_enabled,
                upgrade_status: None,
                running: HashSet::new(),
                last_seen: now_ts(),
                sender: tx.clone(),
            },
        );
        runtime.agent_metadata.insert(
            agent_id.clone(),
            AgentMetadata {
                computer_name: Some(computer_name),
                username: Some(username),
                ip: Some(ip),
                platform: Some(platform),
                arch: Some(arch),
                version: Some(version),
            },
        );
    }

    tx.send(ServerToAgent::HelloAccepted {
        heartbeat_sec: state.server.agent_heartbeat_sec,
    })?;
    state.dispatch_queued()?;
    state.broadcast_state();

    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let Ok(text) = serde_json::to_string(&message) else {
                continue;
            };
            if ws_tx.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = ws_rx.next().await {
        let message = message?;
        match message {
            Message::Text(text) => {
                if let Err(err) = handle_agent_message(&state, &agent_id, &text).await {
                    warn!(agent = %agent_id, %err, "bad agent message");
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    writer.abort();
    disconnect_agent(&state, &agent_id)?;
    Ok(())
}

async fn handle_agent_message(state: &AppState, agent_id: &str, text: &str) -> anyhow::Result<()> {
    let message: AgentToServer = serde_json::from_str(text)?;
    match message {
        AgentToServer::Hello { .. } => {}
        AgentToServer::Heartbeat {
            running,
            capacity,
            runs,
        } => {
            let mut runtime = state
                .runtime
                .lock()
                .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
            if let Some(agent) = runtime.agents.get_mut(agent_id) {
                agent.last_seen = now_ts();
                agent.concurrency = capacity.max(1);
                agent.running = runs.into_iter().map(|run| run.run_id).collect();
                if agent.running.len() != running {
                    warn!(
                        agent = %agent_id,
                        running,
                        reported = agent.running.len(),
                        "agent heartbeat running count mismatch"
                    );
                }
            }
            drop(runtime);
            state.dispatch_queued()?;
            state.broadcast_state();
        }
        AgentToServer::RunStatus {
            run_id,
            state: run_state,
        } => {
            state.storage.update_run_status(&run_id, &run_state)?;
            state.broadcast_state();
        }
        AgentToServer::RunLog {
            run_id,
            stream,
            seq: _,
            data,
        } => {
            let bytes = BASE64.decode(data)?;
            state
                .storage
                .append_log(&run_id, &stream.to_string(), &bytes)?;
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let _ = state.ui_tx.send(UiMessage::Log { run_id, data: text });
        }
        AgentToServer::RunFinished { run_id, exit_code } => {
            state.storage.finish_run(&run_id, exit_code)?;
            {
                let mut runtime = state
                    .runtime
                    .lock()
                    .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
                if let Some(agent) = runtime.agents.get_mut(agent_id) {
                    agent.running.remove(&run_id);
                    agent.last_seen = now_ts();
                }
            }
            state.dispatch_queued()?;
            state.broadcast_state();
        }
        AgentToServer::RunDeleted {
            run_id,
            success,
            error,
        } => {
            let pending = {
                let mut runtime = state
                    .runtime
                    .lock()
                    .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
                runtime.pending_deletes.remove(&run_id)
            };
            let Some(pending) = pending else {
                warn!(agent = %agent_id, run = %run_id, "received unexpected run delete response");
                return Ok(());
            };
            if pending.agent_id != agent_id {
                let _ = pending.sender.send(Err(format!(
                    "delete response came from {}, expected {}",
                    agent_id, pending.agent_id
                )));
                return Ok(());
            }
            let result = if success {
                Ok(())
            } else {
                Err(error.unwrap_or_else(|| "agent failed to delete run workspace".to_owned()))
            };
            let _ = pending.sender.send(result);
        }
        AgentToServer::TerminalStarted { session_id } => {
            state.send_terminal_ui(
                agent_id,
                &session_id,
                TerminalUiMessage::Open {
                    session_id: session_id.clone(),
                    agent_id: agent_id.to_owned(),
                },
            )?;
        }
        AgentToServer::TerminalOutput { session_id, data } => {
            state.send_terminal_ui(agent_id, &session_id, TerminalUiMessage::Output { data })?;
        }
        AgentToServer::TerminalExit {
            session_id,
            exit_code,
            message,
        } => {
            state.finish_terminal_session(
                agent_id,
                &session_id,
                TerminalUiMessage::Exit { exit_code, message },
            )?;
        }
        AgentToServer::UpgradeStatus {
            upgrade_id,
            state: upgrade_state,
            message,
        } => {
            {
                let mut runtime = state
                    .runtime
                    .lock()
                    .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
                if let Some(agent) = runtime.agents.get_mut(agent_id) {
                    agent.last_seen = now_ts();
                    agent.upgrade_status = Some(match &message {
                        Some(message) if !message.is_empty() => {
                            format!("{upgrade_state}: {message}")
                        }
                        _ => upgrade_state.clone(),
                    });
                }
            }
            let log = match message {
                Some(message) if !message.is_empty() => {
                    format!("[{agent_id}] {upgrade_state}: {message}\n")
                }
                _ => format!("[{agent_id}] {upgrade_state}\n"),
            };
            let _ = state.ui_tx.send(UiMessage::UpgradeLog {
                agent_id: agent_id.to_owned(),
                upgrade_id,
                stream: None,
                seq: None,
                data: log,
            });
            state.broadcast_state();
        }
        AgentToServer::UpgradeLog {
            upgrade_id,
            stream,
            seq,
            data,
        } => {
            let bytes = BASE64.decode(data)?;
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let _ = state.ui_tx.send(UiMessage::UpgradeLog {
                agent_id: agent_id.to_owned(),
                upgrade_id,
                stream: Some(stream),
                seq: Some(seq),
                data: text,
            });
        }
    }
    Ok(())
}

fn disconnect_agent(state: &AppState, agent_id: &str) -> anyhow::Result<()> {
    let (pending_deletes, terminal_sessions) = {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        runtime.agents.remove(agent_id);
        let pending_ids = runtime
            .pending_deletes
            .iter()
            .filter_map(|(run_id, pending)| (pending.agent_id == agent_id).then(|| run_id.clone()))
            .collect::<Vec<_>>();
        let pending_deletes = pending_ids
            .into_iter()
            .filter_map(|run_id| runtime.pending_deletes.remove(&run_id))
            .collect::<Vec<_>>();
        let terminal_ids = runtime
            .terminal_sessions
            .iter()
            .filter_map(|(session_id, session)| {
                (session.agent_id == agent_id).then(|| session_id.clone())
            })
            .collect::<Vec<_>>();
        let terminal_sessions = terminal_ids
            .into_iter()
            .filter_map(|session_id| runtime.terminal_sessions.remove(&session_id))
            .collect::<Vec<_>>();
        (pending_deletes, terminal_sessions)
    };
    for pending in pending_deletes {
        let _ = pending.sender.send(Err("agent disconnected".to_owned()));
    }
    for session in terminal_sessions {
        let _ = session.sender.send(TerminalUiMessage::Exit {
            exit_code: None,
            message: Some("agent disconnected".to_owned()),
        });
    }
    let lost = state.storage.mark_agent_runs_lost(agent_id)?;
    if !lost.is_empty() {
        warn!(agent = %agent_id, count = lost.len(), "marked runs lost after agent disconnect");
    }
    state.dispatch_queued()?;
    state.broadcast_state();
    Ok(())
}

async fn ui_ws(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ui_ws(state, socket))
}

async fn handle_ui_ws(state: AppState, socket: WebSocket) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    if let Ok(current) = state.ui_state() {
        let _ = ws_tx
            .send(Message::Text(
                serde_json::to_string(&UiMessage::State { state: current })
                    .unwrap_or_default()
                    .into(),
            ))
            .await;
    }

    let mut rx = state.ui_tx.subscribe();
    loop {
        tokio::select! {
            message = rx.recv() => {
                match message {
                    Ok(message) => {
                        let Ok(text) = serde_json::to_string(&message) else {
                            continue;
                        };
                        if ws_tx.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if let Ok(current) = state.ui_state() {
                            let _ = ws_tx.send(Message::Text(serde_json::to_string(&UiMessage::State { state: current }).unwrap_or_default().into())).await;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = ws_rx.next() => {
                if incoming.is_none() {
                    break;
                }
            }
        }
    }
}

impl AppState {
    fn accept_agent_token(&self, agent_id: &str, token: &str) -> anyhow::Result<()> {
        let agent_id = agent_id.trim();
        if agent_id.is_empty() {
            bail!("agent sent an empty id");
        }
        let token = token.trim();
        if token.is_empty() {
            bail!("agent {agent_id} sent an empty token");
        }

        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;

        if let Some(existing) = runtime.agent_tokens.get(agent_id) {
            if existing != token {
                bail!("invalid token for agent {agent_id}");
            }
        } else {
            runtime
                .agent_tokens
                .insert(agent_id.to_owned(), token.to_owned());
        }
        Ok(())
    }

    fn verify_agent_token(&self, agent_id: &str, token: &str) -> anyhow::Result<()> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        let expected_token = runtime
            .agent_tokens
            .get(agent_id)
            .with_context(|| format!("agent {agent_id} token is not registered"))?;
        if expected_token != token.trim() {
            bail!("invalid token for agent {agent_id}");
        }
        Ok(())
    }

    fn resolve_targets(&self, target_agents: &str) -> anyhow::Result<Vec<String>> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        let mut selected = BTreeSet::new();
        for agent_id in split_csv(target_agents) {
            if !runtime.agents.contains_key(&agent_id)
                && !runtime.agent_metadata.contains_key(&agent_id)
            {
                bail!("unknown target agent {agent_id}");
            }
            selected.insert(agent_id);
        }

        Ok(selected.into_iter().collect())
    }

    fn dispatch_queued(&self) -> anyhow::Result<()> {
        let queued = self.storage.queued_runs()?;
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;

        for run in queued {
            let Some(agent) = runtime.agents.get_mut(&run.agent_id) else {
                continue;
            };
            if agent.running.len() >= agent.concurrency {
                continue;
            }
            if upgrade_blocks_dispatch(agent.upgrade_status.as_deref()) {
                continue;
            }

            self.storage.mark_run_assigned(&run.id)?;
            agent.running.insert(run.id.clone());
            let source_url = format!(
                "{}/api/runs/{}/source",
                self.server.public_url.trim_end_matches('/'),
                run.id
            );
            if let Err(err) = agent.sender.send(ServerToAgent::RunStart {
                run_id: run.id.clone(),
                build_id: run.build_id.clone(),
                source_url,
                archive_format: run.archive_format,
                script_timeout_sec: run.script_timeout_sec,
            }) {
                warn!(agent = %agent.id, run = %run.id, %err, "failed to send run_start");
                agent.running.remove(&run.id);
                self.storage.update_run_status(&run.id, "queued")?;
            }
        }
        Ok(())
    }

    fn ui_state(&self) -> anyhow::Result<UiState> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        let mut agents = Vec::new();
        let mut agent_ids = BTreeSet::new();
        agent_ids.extend(runtime.agent_metadata.keys().cloned());
        agent_ids.extend(runtime.agents.keys().cloned());

        for agent_id in agent_ids {
            if let Some(live) = runtime.agents.get(&agent_id) {
                agents.push(AgentView {
                    id: agent_id.clone(),
                    computer_name: Some(live.computer_name.clone()),
                    username: Some(live.username.clone()),
                    ip: Some(live.ip.clone()),
                    platform: Some(live.platform.clone()),
                    arch: Some(live.arch.clone()),
                    version: Some(live.version.clone()),
                    status: if live.running.is_empty() {
                        "idle".to_owned()
                    } else {
                        "busy".to_owned()
                    },
                    running: live.running.len(),
                    capacity: live.concurrency,
                    current_runs: live.running.iter().cloned().collect(),
                    last_seen: Some(live.last_seen),
                    terminal_enabled: self.server.terminal_enabled && live.terminal_enabled,
                    upgrade_enabled: self.server.upgrade_enabled && live.upgrade_enabled,
                    upgrade_status: live.upgrade_status.clone(),
                });
            } else {
                agents.push(AgentView {
                    id: agent_id.clone(),
                    computer_name: runtime
                        .agent_metadata
                        .get(&agent_id)
                        .and_then(|metadata| metadata.computer_name.clone()),
                    username: runtime
                        .agent_metadata
                        .get(&agent_id)
                        .and_then(|metadata| metadata.username.clone()),
                    ip: runtime
                        .agent_metadata
                        .get(&agent_id)
                        .and_then(|metadata| metadata.ip.clone()),
                    platform: runtime
                        .agent_metadata
                        .get(&agent_id)
                        .and_then(|metadata| metadata.platform.clone()),
                    arch: runtime
                        .agent_metadata
                        .get(&agent_id)
                        .and_then(|metadata| metadata.arch.clone()),
                    version: runtime
                        .agent_metadata
                        .get(&agent_id)
                        .and_then(|metadata| metadata.version.clone()),
                    status: "offline".to_owned(),
                    running: 0,
                    capacity: 0,
                    current_runs: Vec::new(),
                    last_seen: None,
                    terminal_enabled: false,
                    upgrade_enabled: false,
                    upgrade_status: None,
                });
            }
        }
        agents.sort_by(|a, b| {
            let a_online = a.status != "offline";
            let b_online = b.status != "offline";
            b_online
                .cmp(&a_online)
                .then_with(|| {
                    a.computer_name
                        .as_deref()
                        .unwrap_or(&a.id)
                        .to_ascii_lowercase()
                        .cmp(
                            &b.computer_name
                                .as_deref()
                                .unwrap_or(&b.id)
                                .to_ascii_lowercase(),
                        )
                })
                .then_with(|| a.id.cmp(&b.id))
        });

        Ok(UiState {
            agents,
            builds: self.storage.list_builds()?,
            runs: self.storage.list_runs()?,
        })
    }

    fn broadcast_state(&self) {
        match self.ui_state() {
            Ok(state) => {
                let _ = self.ui_tx.send(UiMessage::State { state });
            }
            Err(err) => error!(%err, "failed to build ui state"),
        }
    }

    fn remove_pending_delete(&self, run_id: &str) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.pending_deletes.remove(run_id);
        }
    }

    fn send_terminal_to_agent(
        &self,
        session_id: &str,
        message: ServerToAgent,
    ) -> anyhow::Result<()> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        let session = runtime
            .terminal_sessions
            .get(session_id)
            .with_context(|| format!("terminal session {session_id} is not active"))?;
        let agent = runtime
            .agents
            .get(&session.agent_id)
            .with_context(|| format!("agent {} is offline", session.agent_id))?;
        agent.sender.send(message)?;
        Ok(())
    }

    fn send_terminal_ui(
        &self,
        agent_id: &str,
        session_id: &str,
        message: TerminalUiMessage,
    ) -> anyhow::Result<()> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        let session = runtime
            .terminal_sessions
            .get(session_id)
            .with_context(|| format!("terminal session {session_id} is not active"))?;
        if session.agent_id != agent_id {
            bail!(
                "terminal session {session_id} belongs to {}, not {agent_id}",
                session.agent_id
            );
        }
        let _ = session.sender.send(message);
        Ok(())
    }

    fn finish_terminal_session(
        &self,
        agent_id: &str,
        session_id: &str,
        message: TerminalUiMessage,
    ) -> anyhow::Result<()> {
        let session = {
            let mut runtime = self
                .runtime
                .lock()
                .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
            let Some(session) = runtime.terminal_sessions.remove(session_id) else {
                return Ok(());
            };
            if session.agent_id != agent_id {
                bail!(
                    "terminal session {session_id} belongs to {}, not {agent_id}",
                    session.agent_id
                );
            }
            session
        };
        let _ = session.sender.send(message);
        Ok(())
    }

    fn remove_terminal_session(&self, session_id: &str) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.terminal_sessions.remove(session_id);
        }
    }

    fn close_terminal_session(&self, session_id: &str) {
        let (agent_id, agent_sender) = {
            let mut runtime = match self.runtime.lock() {
                Ok(runtime) => runtime,
                Err(_) => return,
            };
            let Some(session) = runtime.terminal_sessions.remove(session_id) else {
                return;
            };
            let sender = runtime
                .agents
                .get(&session.agent_id)
                .map(|agent| agent.sender.clone());
            (session.agent_id, sender)
        };
        if let Some(sender) = agent_sender {
            let _ = sender.send(ServerToAgent::TerminalClose {
                session_id: session_id.to_owned(),
            });
        } else {
            warn!(agent = %agent_id, session = %session_id, "terminal agent already offline");
        }
    }
}

fn is_terminal_run_status(status: &str) -> bool {
    matches!(status, "success" | "failed" | "canceled" | "lost")
}

fn upgrade_blocks_dispatch(status: Option<&str>) -> bool {
    let Some(status) = status else {
        return false;
    };
    !(status.starts_with("failed")
        || status.starts_with("restart_requested")
        || status.starts_with("success"))
}

fn truncate_close_reason(value: &str) -> String {
    let mut reason = String::new();
    for ch in value.chars() {
        if reason.len() + ch.len_utf8() > 120 {
            break;
        }
        reason.push(ch);
    }
    reason
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn sanitize_filename(filename: &str) -> String {
    let mut value = filename
        .chars()
        .map(|ch| match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-' => ch,
            _ => '_',
        })
        .collect::<String>();
    while value.starts_with('.') {
        value.remove(0);
    }
    if value.is_empty() {
        "package".to_owned()
    } else {
        value
    }
}

fn validate_upgrade_filename(kind: UpgradePackageKind, filename: &str) -> anyhow::Result<()> {
    let lower = filename.to_ascii_lowercase();
    let valid = match kind {
        UpgradePackageKind::Deb => lower.ends_with(".deb"),
        UpgradePackageKind::Rpm => lower.ends_with(".rpm"),
        UpgradePackageKind::Emerge => lower.ends_with(".tar.gz") || lower.ends_with(".tgz"),
    };
    if !valid {
        bail!(
            "upgrade package {} does not match selected kind {}",
            filename,
            kind
        );
    }
    Ok(())
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> anyhow::Result<&'a str> {
    headers
        .get(name)
        .context("missing header")?
        .to_str()
        .with_context(|| format!("invalid header {name}"))
}

struct JsonResponse<T>(T);

impl<T> IntoResponse for JsonResponse<T>
where
    T: serde::Serialize,
{
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(bytes) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                bytes,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                err.to_string(),
            )
                .into_response(),
        }
    }
}

struct ApiError(anyhow::Error);

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        warn!(error = %self.0, "api error");
        (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            self.0.to_string(),
        )
            .into_response()
    }
}

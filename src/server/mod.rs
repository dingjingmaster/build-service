use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, bail};
use axum::{
    Router,
    body::Body,
    extract::{
        DefaultBodyLimit, Multipart, Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    fs,
    io::AsyncWriteExt,
    net::TcpListener,
    sync::{broadcast, mpsc},
    time,
};
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

use crate::{
    config::{CoreConfig, ServerAgentConfig, ServerConfig},
    ids,
    protocol::{AgentToServer, AgentView, ArchiveFormat, ServerToAgent, UiMessage, UiState},
    storage::{NewRun, Storage, now_ts},
};

mod ui;

#[derive(Clone)]
struct AppState {
    server: ServerConfig,
    storage: Storage,
    runtime: Arc<Mutex<RuntimeState>>,
    ui_tx: broadcast::Sender<UiMessage>,
}

struct RuntimeState {
    agents: HashMap<String, AgentRuntime>,
    expected_agents: BTreeMap<String, ServerAgentConfig>,
    agent_metadata: HashMap<String, AgentMetadata>,
}

struct AgentMetadata {
    computer_name: Option<String>,
    username: Option<String>,
    ip: Option<String>,
}

struct AgentRuntime {
    name: String,
    computer_name: String,
    username: String,
    ip: String,
    labels: Vec<String>,
    platform: String,
    arch: String,
    version: String,
    concurrency: usize,
    running: HashSet<String>,
    last_seen: i64,
    sender: mpsc::UnboundedSender<ServerToAgent>,
}

pub async fn run(
    core: CoreConfig,
    server: ServerConfig,
    agents: BTreeMap<String, ServerAgentConfig>,
) -> anyhow::Result<()> {
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
            expected_agents: agents,
            agent_metadata: HashMap::new(),
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
        .route("/api/runs/{run_id}/source", get(download_source))
        .route("/api/runs/{run_id}/log", get(read_log))
        .route("/api/runs/{run_id}/rerun", post(rerun))
        .route("/api/runs/{run_id}/cancel", post(cancel))
        .route("/api/agents/{agent_name}", delete(delete_agent))
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
                    .map(|agent| agent.name.clone())
                    .collect::<Vec<_>>()
            };

            for agent_name in stale_agents {
                if let Err(err) = disconnect_agent(&state, &agent_name) {
                    warn!(agent = %agent_name, %err, "failed to disconnect stale agent");
                }
            }
        }
    });
}

async fn index() -> Html<&'static str> {
    Html(ui::INDEX_HTML)
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
    let mut target_agents = String::new();
    let mut target_labels = String::new();

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
                target_agents = field.text().await.unwrap_or_default();
            }
            "target_labels" => {
                target_labels = field.text().await.unwrap_or_default();
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
    let selected = state.resolve_targets(&target_agents, &target_labels)?;
    if selected.is_empty() {
        return Err(anyhow::anyhow!("no target agents selected").into());
    }

    let runs: Vec<NewRun> = selected
        .into_iter()
        .map(|agent| NewRun {
            id: ids::run_id(),
            agent_name: agent.name,
            labels: agent.labels,
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

async fn download_source(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let agent_name = header_value(&headers, "x-agent-name").context("missing x-agent-name")?;
    let token = header_value(&headers, "x-agent-token").context("missing x-agent-token")?;
    state.verify_agent_token(agent_name, token)?;

    let run = state.storage.get_run(&run_id)?.context("run not found")?;
    if run.agent_name != agent_name {
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
        if let Some(agent) = runtime.agents.get(&run.agent_name) {
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

async fn delete_agent(
    State(state): State<AppState>,
    Path(agent_name): Path<String>,
) -> Result<JsonResponse<UiState>, ApiError> {
    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        if runtime.agents.contains_key(&agent_name) {
            return Err(anyhow::anyhow!("agent {agent_name} is online").into());
        }
        if runtime.expected_agents.remove(&agent_name).is_none() {
            return Err(anyhow::anyhow!("agent {agent_name} not found").into());
        }
        runtime.agent_metadata.remove(&agent_name);
    }
    state.broadcast_state();
    Ok(JsonResponse(state.ui_state()?))
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
        name,
        token,
        computer_name,
        username,
        ip,
        labels,
        platform,
        arch,
        concurrency,
        version,
    } = hello
    else {
        bail!("first agent message must be hello");
    };
    state.verify_agent_token(&name, &token)?;

    let (tx, mut rx) = mpsc::unbounded_channel::<ServerToAgent>();
    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        runtime.agents.insert(
            name.clone(),
            AgentRuntime {
                name: name.clone(),
                computer_name: computer_name.clone(),
                username: username.clone(),
                ip: ip.clone(),
                labels,
                platform,
                arch,
                version,
                concurrency: concurrency.max(1),
                running: HashSet::new(),
                last_seen: now_ts(),
                sender: tx.clone(),
            },
        );
        runtime.agent_metadata.insert(
            name.clone(),
            AgentMetadata {
                computer_name: Some(computer_name),
                username: Some(username),
                ip: Some(ip),
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
                if let Err(err) = handle_agent_message(&state, &name, &text).await {
                    warn!(agent = %name, %err, "bad agent message");
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    writer.abort();
    disconnect_agent(&state, &name)?;
    Ok(())
}

async fn handle_agent_message(
    state: &AppState,
    agent_name: &str,
    text: &str,
) -> anyhow::Result<()> {
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
            if let Some(agent) = runtime.agents.get_mut(agent_name) {
                agent.last_seen = now_ts();
                agent.concurrency = capacity.max(1);
                agent.running = runs.into_iter().map(|run| run.run_id).collect();
                if agent.running.len() != running {
                    warn!(
                        agent = %agent_name,
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
                if let Some(agent) = runtime.agents.get_mut(agent_name) {
                    agent.running.remove(&run_id);
                    agent.last_seen = now_ts();
                }
            }
            state.dispatch_queued()?;
            state.broadcast_state();
        }
    }
    Ok(())
}

fn disconnect_agent(state: &AppState, agent_name: &str) -> anyhow::Result<()> {
    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        runtime.agents.remove(agent_name);
    }
    let lost = state.storage.mark_agent_runs_lost(agent_name)?;
    if !lost.is_empty() {
        warn!(agent = %agent_name, count = lost.len(), "marked runs lost after agent disconnect");
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
    fn verify_agent_token(&self, name: &str, token: &str) -> anyhow::Result<()> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        let expected = runtime
            .expected_agents
            .get(name)
            .with_context(|| format!("unknown agent {name}"))?;
        if !expected.enabled {
            bail!("agent {name} is disabled");
        }
        if expected.token != token {
            bail!("invalid token for agent {name}");
        }
        Ok(())
    }

    fn resolve_targets(
        &self,
        target_agents: &str,
        target_labels: &str,
    ) -> anyhow::Result<Vec<ServerAgentConfig>> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;
        let mut selected = BTreeSet::new();
        for name in split_csv(target_agents) {
            if !runtime.expected_agents.contains_key(&name) {
                bail!("unknown target agent {name}");
            }
            selected.insert(name);
        }

        let labels = split_csv(target_labels);
        if !labels.is_empty() {
            for agent in runtime.expected_agents.values() {
                if agent.enabled && labels.iter().all(|label| agent.labels.contains(label)) {
                    selected.insert(agent.name.clone());
                }
            }
        }

        Ok(selected
            .into_iter()
            .filter_map(|name| runtime.expected_agents.get(&name).cloned())
            .filter(|agent| agent.enabled)
            .collect())
    }

    fn dispatch_queued(&self) -> anyhow::Result<()> {
        let queued = self.storage.queued_runs()?;
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime poisoned"))?;

        for run in queued {
            let Some(agent) = runtime.agents.get_mut(&run.agent_name) else {
                continue;
            };
            if agent.running.len() >= agent.concurrency {
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
                warn!(agent = %agent.name, run = %run.id, %err, "failed to send run_start");
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

        for expected in runtime.expected_agents.values() {
            if let Some(live) = runtime.agents.get(&expected.name) {
                agents.push(AgentView {
                    name: expected.name.clone(),
                    computer_name: Some(live.computer_name.clone()),
                    username: Some(live.username.clone()),
                    ip: Some(live.ip.clone()),
                    labels: live.labels.clone(),
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
                    enabled: expected.enabled,
                });
            } else {
                agents.push(AgentView {
                    name: expected.name.clone(),
                    computer_name: runtime
                        .agent_metadata
                        .get(&expected.name)
                        .and_then(|metadata| metadata.computer_name.clone()),
                    username: runtime
                        .agent_metadata
                        .get(&expected.name)
                        .and_then(|metadata| metadata.username.clone()),
                    ip: runtime
                        .agent_metadata
                        .get(&expected.name)
                        .and_then(|metadata| metadata.ip.clone()),
                    labels: expected.labels.clone(),
                    platform: None,
                    arch: None,
                    version: None,
                    status: "offline".to_owned(),
                    running: 0,
                    capacity: 0,
                    current_runs: Vec::new(),
                    last_seen: None,
                    enabled: expected.enabled,
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
                        .unwrap_or(&a.name)
                        .to_ascii_lowercase()
                        .cmp(
                            &b.computer_name
                                .as_deref()
                                .unwrap_or(&b.name)
                                .to_ascii_lowercase(),
                        )
                })
                .then_with(|| a.name.cmp(&b.name))
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
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
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

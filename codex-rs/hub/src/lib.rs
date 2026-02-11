#![deny(clippy::print_stdout, clippy::print_stderr)]

use anyhow::Context;
use anyhow::Result;
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use futures::StreamExt;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio::time::sleep;
use tracing::warn;

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 46_710;

const START_WAIT_ATTEMPTS: usize = 50;
const START_WAIT_DELAY: Duration = Duration::from_millis(100);
const PAIR_CODE_LENGTH: usize = 8;
const PAIR_CODE_TTL_SECONDS: i64 = 10 * 60;

fn default_lan_bind_host() -> String {
    DEFAULT_HOST.to_string()
}

const fn default_lan_bind_port() -> u16 {
    DEFAULT_PORT
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartOptions {
    pub codex_bin: PathBuf,
    pub codex_home: PathBuf,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonOptions {
    pub codex_home: PathBuf,
    pub host: String,
    pub port: u16,
    pub auth_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingCode {
    pub code: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceRole {
    Observer,
    Operator,
    PromptOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubDevice {
    pub device_id: String,
    pub device_name: String,
    pub role: DeviceRole,
    pub paired_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairCompleteResult {
    pub device: HubDevice,
    pub device_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanConfig {
    pub enabled: bool,
    pub bind_host: String,
    pub bind_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubStatus {
    pub running: bool,
    pub endpoint: Option<String>,
    pub pid: Option<u32>,
    pub started_at: Option<i64>,
    pub lan_enabled: bool,
    pub device_count: usize,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HubStateRecord {
    pid: u32,
    host: String,
    port: u16,
    started_at: i64,
    auth_token: String,
    #[serde(default)]
    pairing: Option<PairingCode>,
    #[serde(default)]
    lan_enabled: bool,
    #[serde(default = "default_lan_bind_host")]
    lan_bind_host: String,
    #[serde(default = "default_lan_bind_port")]
    lan_bind_port: u16,
    #[serde(default)]
    devices: Vec<HubDeviceRecord>,
    #[serde(default)]
    pinned_prompts: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HubDeviceRecord {
    device: HubDevice,
    device_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: String,
    pid: u32,
    started_at: i64,
    host: String,
    port: u16,
    lan_enabled: bool,
    device_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairStartResponse {
    code: String,
    expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairCompleteRequest {
    code: String,
    device_name: String,
    role: DeviceRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairCompleteResponse {
    device: HubDevice,
    device_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceListResponse {
    data: Vec<HubDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceRevokeRequest {
    device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceRevokeResponse {
    revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigureLanRequest {
    enabled: bool,
    bind_host: Option<String>,
    bind_port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigureLanResponse {
    enabled: bool,
    bind_host: String,
    bind_port: u16,
}

#[derive(Clone)]
struct DaemonState {
    inner: Arc<Mutex<HubStateRecord>>,
    state_path: PathBuf,
    event_tx: broadcast::Sender<String>,
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

pub fn runtime_dir(codex_home: &Path) -> PathBuf {
    codex_home.join("hub")
}

pub fn state_path(codex_home: &Path) -> PathBuf {
    runtime_dir(codex_home).join("state.json")
}

pub fn endpoint(host: &str, port: u16) -> String {
    format!("http://{host}:{port}")
}

pub async fn start_daemon(options: StartOptions) -> Result<HubStatus> {
    let existing_status = status(&options.codex_home).await?;
    if existing_status.running {
        anyhow::bail!(
            "codex hub is already running at {}",
            existing_status
                .endpoint
                .unwrap_or_else(|| endpoint(&options.host, options.port))
        );
    }

    if existing_status.endpoint.is_some() {
        remove_state_if_exists(&options.codex_home).await?;
    }

    let auth_token = random_hex_token();
    spawn_daemon_process(&options, &auth_token)?;

    let mut attempts = START_WAIT_ATTEMPTS;
    while attempts > 0 {
        let current_status = status(&options.codex_home).await?;
        if current_status.running {
            return Ok(current_status);
        }
        attempts -= 1;
        sleep(START_WAIT_DELAY).await;
    }

    anyhow::bail!("timed out waiting for codex hub to start")
}

pub async fn run_daemon(options: DaemonOptions) -> Result<()> {
    let state_path = state_path(&options.codex_home);
    let mut record = HubStateRecord {
        pid: std::process::id(),
        host: options.host,
        port: options.port,
        started_at: now_unix_seconds(),
        auth_token: options.auth_token,
        pairing: None,
        lan_enabled: false,
        lan_bind_host: default_lan_bind_host(),
        lan_bind_port: default_lan_bind_port(),
        devices: Vec::new(),
        pinned_prompts: HashMap::new(),
    };

    let listener = tokio::net::TcpListener::bind((record.host.as_str(), record.port))
        .await
        .with_context(|| {
            format!(
                "failed to bind codex hub on {}",
                endpoint(&record.host, record.port)
            )
        })?;
    let local_addr = listener
        .local_addr()
        .context("failed to read hub local address")?;
    record.port = local_addr.port();
    write_state_record(&state_path, &record).await?;

    let (event_tx, _) = broadcast::channel(256);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let daemon_state = DaemonState {
        inner: Arc::new(Mutex::new(record)),
        state_path: state_path.clone(),
        event_tx,
        shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
    };
    let router = Router::new()
        .route("/health", get(health_handler))
        .route("/ws", get(ws_handler))
        .route("/pair/start", post(pair_start_handler))
        .route("/pair/complete", post(pair_complete_handler))
        .route("/device/list", get(device_list_handler))
        .route("/device/revoke", post(device_revoke_handler))
        .route("/hub/configure-lan", post(configure_lan_handler))
        .route("/admin/shutdown", post(shutdown_handler))
        .with_state(daemon_state);

    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        tokio::select! {
            _ = &mut shutdown_rx => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    });

    let result = server.await.context("codex hub server failed");
    remove_state_if_exists(&options.codex_home).await?;
    result
}

pub async fn status(codex_home: &Path) -> Result<HubStatus> {
    let Some(state) = read_state_optional(codex_home).await? else {
        return Ok(HubStatus {
            running: false,
            endpoint: None,
            pid: None,
            started_at: None,
            lan_enabled: false,
            device_count: 0,
            reason: None,
        });
    };

    let health_url = format!("{}/health", endpoint(&state.host, state.port));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(400))
        .build()
        .context("failed to build hub status client")?;
    match client.get(health_url).send().await {
        Ok(response) => {
            let response = response.error_for_status();
            match response {
                Ok(ok_response) => {
                    let health: HealthResponse = ok_response
                        .json()
                        .await
                        .context("failed to parse hub health response")?;
                    Ok(HubStatus {
                        running: true,
                        endpoint: Some(endpoint(&health.host, health.port)),
                        pid: Some(health.pid),
                        started_at: Some(health.started_at),
                        lan_enabled: health.lan_enabled,
                        device_count: health.device_count,
                        reason: None,
                    })
                }
                Err(err) => Ok(HubStatus {
                    running: false,
                    endpoint: Some(endpoint(&state.host, state.port)),
                    pid: Some(state.pid),
                    started_at: Some(state.started_at),
                    lan_enabled: state.lan_enabled,
                    device_count: state.devices.len(),
                    reason: Some(format!("hub returned unhealthy status: {err}")),
                }),
            }
        }
        Err(err) => Ok(HubStatus {
            running: false,
            endpoint: Some(endpoint(&state.host, state.port)),
            pid: Some(state.pid),
            started_at: Some(state.started_at),
            lan_enabled: state.lan_enabled,
            device_count: state.devices.len(),
            reason: Some(format!("hub not reachable: {err}")),
        }),
    }
}

pub async fn stop_daemon(codex_home: &Path) -> Result<HubStatus> {
    let Some(state) = read_state_optional(codex_home).await? else {
        return Ok(HubStatus {
            running: false,
            endpoint: None,
            pid: None,
            started_at: None,
            lan_enabled: false,
            device_count: 0,
            reason: Some("codex hub is not running".to_string()),
        });
    };

    let shutdown_url = format!("{}/admin/shutdown", endpoint(&state.host, state.port));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .context("failed to build hub shutdown client")?;
    let response = client
        .post(shutdown_url)
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", state.auth_token),
        )
        .send()
        .await;

    if let Err(err) = response {
        warn!("hub shutdown request failed: {err}");
        remove_state_if_exists(codex_home).await?;
        return Ok(HubStatus {
            running: false,
            endpoint: Some(endpoint(&state.host, state.port)),
            pid: Some(state.pid),
            started_at: Some(state.started_at),
            lan_enabled: state.lan_enabled,
            device_count: state.devices.len(),
            reason: Some("hub was unreachable; removed stale state".to_string()),
        });
    }

    let mut attempts = 30;
    while attempts > 0 {
        let current_status = status(codex_home).await?;
        if !current_status.running {
            remove_state_if_exists(codex_home).await?;
            return Ok(current_status);
        }
        attempts -= 1;
        sleep(Duration::from_millis(100)).await;
    }

    anyhow::bail!("timed out waiting for codex hub to stop")
}

pub async fn start_pairing(codex_home: &Path) -> Result<PairingCode> {
    let Some(state) = read_state_optional(codex_home).await? else {
        anyhow::bail!("codex hub is not running")
    };
    let pairing_url = format!("{}/pair/start", endpoint(&state.host, state.port));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .context("failed to build hub pairing client")?;
    let response = client
        .post(pairing_url)
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", state.auth_token),
        )
        .send()
        .await
        .context("failed to request pairing code")?
        .error_for_status()
        .context("hub rejected pairing request")?;
    let pair_response: PairStartResponse = response
        .json()
        .await
        .context("failed to parse pairing response")?;
    Ok(PairingCode {
        code: pair_response.code,
        expires_at: pair_response.expires_at,
    })
}

pub async fn complete_pairing(
    codex_home: &Path,
    code: String,
    device_name: String,
    role: DeviceRole,
) -> Result<PairCompleteResult> {
    let mut state = read_state_required(codex_home).await?;
    let now = now_unix_seconds();
    let Some(pairing) = state.pairing.clone() else {
        anyhow::bail!("no active pairing request")
    };
    if pairing.expires_at < now {
        anyhow::bail!("pairing code has expired")
    }
    if !pairing.code.eq_ignore_ascii_case(code.trim()) {
        anyhow::bail!("invalid pairing code")
    }

    let device = HubDevice {
        device_id: random_device_id(),
        device_name: device_name.trim().to_string(),
        role,
        paired_at: now,
        last_seen_at: now,
    };
    let device_record = HubDeviceRecord {
        device: device.clone(),
        device_token: random_hex_token(),
    };
    state.devices.push(device_record.clone());
    state.pairing = None;
    write_state_record(&state_path(codex_home), &state).await?;

    Ok(PairCompleteResult {
        device,
        device_token: device_record.device_token,
    })
}

pub async fn list_devices(codex_home: &Path) -> Result<Vec<HubDevice>> {
    let state = read_state_required(codex_home).await?;
    Ok(state
        .devices
        .into_iter()
        .map(|device| device.device)
        .collect())
}

pub async fn revoke_device(codex_home: &Path, device_id: &str) -> Result<bool> {
    let mut state = read_state_required(codex_home).await?;
    let original_len = state.devices.len();
    state
        .devices
        .retain(|record| record.device.device_id != device_id);
    let revoked = state.devices.len() != original_len;
    if revoked {
        write_state_record(&state_path(codex_home), &state).await?;
    }
    Ok(revoked)
}

pub async fn configure_lan(
    codex_home: &Path,
    enabled: bool,
    bind_host: Option<String>,
    bind_port: Option<u16>,
) -> Result<LanConfig> {
    let mut state = read_state_required(codex_home).await?;
    state.lan_enabled = enabled;
    if let Some(bind_host) = bind_host {
        let bind_host = bind_host.trim();
        if !bind_host.is_empty() {
            state.lan_bind_host = bind_host.to_string();
        }
    }
    if let Some(bind_port) = bind_port {
        state.lan_bind_port = bind_port;
    }
    let response = LanConfig {
        enabled: state.lan_enabled,
        bind_host: state.lan_bind_host.clone(),
        bind_port: state.lan_bind_port,
    };
    write_state_record(&state_path(codex_home), &state).await?;
    Ok(response)
}

pub async fn set_pinned_prompt(codex_home: &Path, thread_id: &str, prompt: String) -> Result<()> {
    let mut state = read_state_required(codex_home).await?;
    let thread_id = thread_id.trim();
    if thread_id.is_empty() {
        anyhow::bail!("thread_id cannot be empty");
    }
    if prompt.trim().is_empty() {
        state.pinned_prompts.remove(thread_id);
    } else {
        state.pinned_prompts.insert(thread_id.to_string(), prompt);
    }
    write_state_record(&state_path(codex_home), &state).await
}

pub async fn get_pinned_prompt(codex_home: &Path, thread_id: &str) -> Result<Option<String>> {
    let state = read_state_required(codex_home).await?;
    Ok(state.pinned_prompts.get(thread_id).cloned())
}

fn spawn_daemon_process(options: &StartOptions, auth_token: &str) -> Result<()> {
    let mut command = std::process::Command::new(&options.codex_bin);
    command
        .arg("hub")
        .arg("daemon")
        .arg("--codex-home")
        .arg(&options.codex_home)
        .arg("--host")
        .arg(&options.host)
        .arg("--port")
        .arg(options.port.to_string())
        .arg("--auth-token")
        .arg(auth_token)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }

    command.spawn().with_context(|| {
        format!(
            "failed to spawn codex hub from {}",
            options.codex_bin.display()
        )
    })?;
    Ok(())
}

async fn read_state_optional(codex_home: &Path) -> Result<Option<HubStateRecord>> {
    let path = state_path(codex_home);
    let raw = match fs::read_to_string(path).await {
        Ok(raw) => raw,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let state: HubStateRecord =
        serde_json::from_str(&raw).context("failed to deserialize codex hub state file")?;
    Ok(Some(state))
}

async fn read_state_required(codex_home: &Path) -> Result<HubStateRecord> {
    read_state_optional(codex_home)
        .await?
        .context("codex hub state file was not found; start the hub first")
}

async fn write_state_record(path: &Path, state: &HubStateRecord) -> Result<()> {
    let runtime_dir = path
        .parent()
        .context("invalid hub state path without parent")?;
    fs::create_dir_all(runtime_dir).await?;

    let payload = serde_json::to_vec_pretty(state).context("failed to serialize hub state")?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, payload).await?;

    if let Err(err) = fs::rename(&temp_path, path).await {
        if err.kind() == ErrorKind::AlreadyExists {
            fs::remove_file(path).await?;
            fs::rename(&temp_path, path).await?;
        } else {
            return Err(err.into());
        }
    }
    Ok(())
}

async fn remove_state_if_exists(codex_home: &Path) -> Result<()> {
    let path = state_path(codex_home);
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn now_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn random_hex_token() -> String {
    let bytes: [u8; 24] = rand::random();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn random_device_id() -> String {
    let bytes: [u8; 10] = rand::random();
    let suffix: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("dev-{suffix}")
}

fn random_pair_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..PAIR_CODE_LENGTH)
        .map(|_| {
            let index = rng.random_range(0..ALPHABET.len());
            ALPHABET[index] as char
        })
        .collect()
}

fn is_authorized(headers: &HeaderMap, token: &str) -> bool {
    let Some(value) = headers.get(header::AUTHORIZATION) else {
        return false;
    };
    let Ok(raw_header) = value.to_str() else {
        return false;
    };
    let Some(supplied_token) = raw_header.strip_prefix("Bearer ") else {
        return false;
    };
    supplied_token == token
}

fn bearer_token_from_headers(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?;
    let raw_header = value.to_str().ok()?;
    raw_header.strip_prefix("Bearer ")
}

fn is_authorized_for_stream(headers: &HeaderMap, state: &HubStateRecord) -> bool {
    let Some(supplied_token) = bearer_token_from_headers(headers) else {
        return false;
    };
    supplied_token == state.auth_token
        || state
            .devices
            .iter()
            .any(|device| device.device_token == supplied_token)
}

fn send_hub_event(state: &DaemonState, event: &str, payload: serde_json::Value) {
    let body = serde_json::json!({
        "event": event,
        "at": now_unix_seconds(),
        "payload": payload,
    });
    if let Ok(encoded) = serde_json::to_string(&body) {
        let _ = state.event_tx.send(encoded);
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<DaemonState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let snapshot = state.inner.lock().await.clone();
    if !is_authorized_for_stream(&headers, &snapshot) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut receiver = state.event_tx.subscribe();
    Ok(ws.on_upgrade(move |mut socket: WebSocket| async move {
        let connected = serde_json::json!({
            "event": "hub.connected",
            "at": now_unix_seconds(),
            "payload": {},
        });
        let _ = socket
            .send(Message::Text(connected.to_string().into()))
            .await;
        loop {
            tokio::select! {
                inbound = socket.next() => {
                    match inbound {
                        Some(Ok(Message::Ping(payload))) => {
                            if socket.send(Message::Pong(payload)).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Ok(_)) => {}
                        Some(Err(_)) => break,
                    }
                }
                outbound = receiver.recv() => {
                    match outbound {
                        Ok(message) => {
                            if socket.send(Message::Text(message.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    }))
}

async fn health_handler(State(state): State<DaemonState>) -> Json<HealthResponse> {
    let state = state.inner.lock().await.clone();
    Json(HealthResponse {
        status: "ok".to_string(),
        pid: state.pid,
        started_at: state.started_at,
        host: state.host,
        port: state.port,
        lan_enabled: state.lan_enabled,
        device_count: state.devices.len(),
    })
}

async fn pair_start_handler(
    State(state): State<DaemonState>,
    headers: HeaderMap,
) -> Result<Json<PairStartResponse>, StatusCode> {
    let current_token = {
        let state_guard = state.inner.lock().await;
        state_guard.auth_token.clone()
    };
    if !is_authorized(&headers, &current_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let pairing_code = PairingCode {
        code: random_pair_code(),
        expires_at: now_unix_seconds() + PAIR_CODE_TTL_SECONDS,
    };
    let updated_state = {
        let mut state_guard = state.inner.lock().await;
        state_guard.pairing = Some(pairing_code.clone());
        state_guard.clone()
    };

    write_state_record(&state.state_path, &updated_state)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    send_hub_event(
        &state,
        "device.pairingRequested",
        serde_json::json!({
            "code": pairing_code.code.clone(),
            "expiresAt": pairing_code.expires_at,
        }),
    );

    Ok(Json(PairStartResponse {
        code: pairing_code.code,
        expires_at: pairing_code.expires_at,
    }))
}

async fn pair_complete_handler(
    State(state): State<DaemonState>,
    Json(payload): Json<PairCompleteRequest>,
) -> Result<Json<PairCompleteResponse>, StatusCode> {
    let mut state_guard = state.inner.lock().await;
    let now = now_unix_seconds();
    let Some(pairing) = state_guard.pairing.clone() else {
        return Err(StatusCode::FORBIDDEN);
    };
    if pairing.expires_at < now || !pairing.code.eq_ignore_ascii_case(payload.code.trim()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let device_name = payload.device_name.trim();
    if device_name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let device = HubDevice {
        device_id: random_device_id(),
        device_name: device_name.to_string(),
        role: payload.role,
        paired_at: now,
        last_seen_at: now,
    };
    let device_token = random_hex_token();
    state_guard.devices.push(HubDeviceRecord {
        device: device.clone(),
        device_token: device_token.clone(),
    });
    state_guard.pairing = None;
    let updated_state = state_guard.clone();
    drop(state_guard);

    write_state_record(&state.state_path, &updated_state)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    send_hub_event(
        &state,
        "device.paired",
        serde_json::json!({
            "deviceId": device.device_id.clone(),
            "deviceName": device.device_name.clone(),
            "role": device.role,
        }),
    );

    Ok(Json(PairCompleteResponse {
        device,
        device_token,
    }))
}

async fn device_list_handler(
    State(state): State<DaemonState>,
    headers: HeaderMap,
) -> Result<Json<DeviceListResponse>, StatusCode> {
    let state_guard = state.inner.lock().await;
    if !is_authorized(&headers, &state_guard.auth_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let data = state_guard
        .devices
        .iter()
        .map(|record| record.device.clone())
        .collect();
    Ok(Json(DeviceListResponse { data }))
}

async fn device_revoke_handler(
    State(state): State<DaemonState>,
    headers: HeaderMap,
    Json(payload): Json<DeviceRevokeRequest>,
) -> Result<Json<DeviceRevokeResponse>, StatusCode> {
    let mut state_guard = state.inner.lock().await;
    if !is_authorized(&headers, &state_guard.auth_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let original_len = state_guard.devices.len();
    state_guard
        .devices
        .retain(|record| record.device.device_id != payload.device_id);
    let revoked = state_guard.devices.len() != original_len;
    let updated_state = state_guard.clone();
    drop(state_guard);

    if revoked {
        write_state_record(&state.state_path, &updated_state)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        send_hub_event(
            &state,
            "device.revoked",
            serde_json::json!({
                "deviceId": payload.device_id,
            }),
        );
    }

    Ok(Json(DeviceRevokeResponse { revoked }))
}

async fn configure_lan_handler(
    State(state): State<DaemonState>,
    headers: HeaderMap,
    Json(payload): Json<ConfigureLanRequest>,
) -> Result<Json<ConfigureLanResponse>, StatusCode> {
    let mut state_guard = state.inner.lock().await;
    if !is_authorized(&headers, &state_guard.auth_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    state_guard.lan_enabled = payload.enabled;
    if let Some(bind_host) = payload.bind_host {
        let bind_host = bind_host.trim();
        if !bind_host.is_empty() {
            state_guard.lan_bind_host = bind_host.to_string();
        }
    }
    if let Some(bind_port) = payload.bind_port {
        state_guard.lan_bind_port = bind_port;
    }

    let response = ConfigureLanResponse {
        enabled: state_guard.lan_enabled,
        bind_host: state_guard.lan_bind_host.clone(),
        bind_port: state_guard.lan_bind_port,
    };
    let updated_state = state_guard.clone();
    drop(state_guard);

    write_state_record(&state.state_path, &updated_state)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    send_hub_event(
        &state,
        "hub.lanConfigured",
        serde_json::json!({
            "enabled": response.enabled,
            "bindHost": response.bind_host.clone(),
            "bindPort": response.bind_port,
        }),
    );

    Ok(Json(response))
}

async fn shutdown_handler(
    State(state): State<DaemonState>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let current_token = {
        let state_guard = state.inner.lock().await;
        state_guard.auth_token.clone()
    };
    if !is_authorized(&headers, &current_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let sender = {
        let mut sender_guard = state.shutdown_tx.lock().await;
        sender_guard.take()
    };
    if let Some(sender) = sender {
        let _ = sender.send(());
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_HOST;
    use super::DaemonOptions;
    use super::PAIR_CODE_LENGTH;
    use super::run_daemon;
    use super::start_pairing;
    use super::status;
    use super::stop_daemon;
    use pretty_assertions::assert_eq;
    use std::time::Duration;

    #[tokio::test]
    async fn daemon_status_pairing_and_shutdown_flow() {
        let codex_home = tempfile::tempdir().expect("create temp codex home");
        let options = DaemonOptions {
            codex_home: codex_home.path().to_path_buf(),
            host: DEFAULT_HOST.to_string(),
            port: 0,
            auth_token: "test-token".to_string(),
        };

        let daemon = tokio::spawn(run_daemon(options));

        let mut attempts = 30;
        while attempts > 0 {
            let hub_status = status(codex_home.path())
                .await
                .expect("status should succeed");
            if hub_status.running {
                break;
            }
            attempts -= 1;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let hub_status = status(codex_home.path())
            .await
            .expect("status should succeed");
        assert_eq!(hub_status.running, true);
        assert_eq!(hub_status.endpoint.is_some(), true);

        let pairing_code = start_pairing(codex_home.path())
            .await
            .expect("pairing should succeed");
        assert_eq!(pairing_code.code.len(), PAIR_CODE_LENGTH);

        let stopped_status = stop_daemon(codex_home.path())
            .await
            .expect("stop should succeed");
        assert_eq!(stopped_status.running, false);

        let daemon_result = daemon.await.expect("join daemon task");
        assert_eq!(daemon_result.is_ok(), true);
    }
}

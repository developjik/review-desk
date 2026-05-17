use reviewdesk::app_core::{
    AgentRunRecordView, AiBlockedReason, AiConnectionStatus, AiConnectionStatusView,
    AiModelView, AiRateLimitSnapshot, AiRateLimitStatus, AppStatusContext, AppStatusView, Locale,
    ReasoningEffort, frontend_safe_app_status_from_context, validate_ai_run_configuration,
    validate_external_url,
};
use reviewdesk::auth::{KeyringTokenStore, TokenKind, TokenStore};
use reviewdesk::codex_bridge::{CodexBridge, CodexBridgeStatusView, sanitize_codex_diagnostics};
use reviewdesk::domain::{
    ChangedFile, GitHubErrorKind, InlineCommentDraft, InlineMappingStatus, PublishAttempt,
    PullRequestQueueItem, Repository, ReviewDeskError, ReviewDraft, ReviewEvent,
    ReviewPublishPayload,
};
use reviewdesk::github::{
    DevicePoll, GitHubClient, PullRequestContextView, ReviewSubmitComment, ReviewSubmitRequest,
    SubmittedReviewResponse, build_github_authorize_url, pkce_challenge_s256,
};
use reviewdesk::inline_comments::validate_inline_comments as validate_inline_comments_core;
use reviewdesk::publish::{
    PreparedReviewPublishStatus, PreparedReviewPublishView, prepare_review_publish,
};
use reviewdesk::review::{ReviewPipeline, build_review_input, stable_changed_files_hash};
use reviewdesk::security::PrivateDiffConsent;
use reviewdesk::storage::LocalStore;
use reviewdesk::workspace_store::{PrWorkspaceKey, WorkspaceStore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{Duration, timeout};

const GITHUB_FLOW_ID: &str = "github-oauth-flow";
const GITHUB_OAUTH_CALLBACK_PATH: &str = "/auth/github/callback";
const GITHUB_OAUTH_EXPIRES_IN_SECONDS: u64 = 600;
const GITHUB_OAUTH_SCOPES: &[&str] = &["read:user", "repo"];

pub struct ReviewDeskTauriState {
    github_flow: Arc<Mutex<Option<GitHubOAuthFlow>>>,
    codex_login: Arc<Mutex<Option<CodexLoginFlow>>>,
    ai_settings: Arc<Mutex<AiLocalSettings>>,
    agent_runs: Arc<Mutex<BTreeMap<String, AgentRunStateView>>>,
}

impl Default for ReviewDeskTauriState {
    fn default() -> Self {
        Self {
            github_flow: Arc::new(Mutex::new(None)),
            codex_login: Arc::new(Mutex::new(None)),
            ai_settings: Arc::new(Mutex::new(AiLocalSettings::default())),
            agent_runs: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

#[derive(Debug, Clone)]
enum GitHubOAuthFlow {
    Device {
        client_id: String,
        device_code: String,
    },
    Browser {
        flow_id: String,
        status: GitHubBrowserOAuthStatus,
        disabled_reason: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GitHubBrowserOAuthStatus {
    CallbackWaiting,
    Authorized,
    Cancelled,
    Denied,
    Timeout,
    Failed,
}

impl GitHubBrowserOAuthStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::CallbackWaiting => "callback_waiting",
            Self::Authorized => "authorized",
            Self::Cancelled => "cancelled",
            Self::Denied => "denied",
            Self::Timeout => "timeout",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
struct CodexLoginFlow {
    login_id: String,
}

#[derive(Debug, Clone)]
struct AiLocalSettings {
    selected_model: String,
    selected_reasoning_effort: ReasoningEffort,
}

impl Default for AiLocalSettings {
    fn default() -> Self {
        Self {
            selected_model: "gpt-5.5".to_string(),
            selected_reasoning_effort: ReasoningEffort::Medium,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<ReviewDeskError> for CommandError {
    fn from(error: ReviewDeskError) -> Self {
        match error {
            ReviewDeskError::GitHubApi { kind, message } => Self {
                code: github_error_code(kind).to_string(),
                message,
            },
            other => Self {
                code: "reviewdesk_error".to_string(),
                message: other.to_string(),
            },
        }
    }
}

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub async fn get_app_status(
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<AppStatusView> {
    let store = KeyringTokenStore::default();
    let github = github_connection_state(&store, &state).await?;
    let settings = state
        .ai_settings
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "AI settings state lock failed"))?
        .clone();
    let ai_connection = codex_ai_connection_status().await;
    let ai_models = codex_ai_models_for(&ai_connection).await;
    Ok(frontend_safe_app_status_from_context(AppStatusContext {
        github,
        ai_connection,
        strict_auth_mode: env::var("REVIEWDESK_STRICT_AUTH_MODE")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        language_preferences: reviewdesk::app_core::LanguagePreferences::for_os_locale(
            &env::var("LANG").unwrap_or_else(|_| "en-US".to_string()),
        ),
        ai_models,
        selected_model: settings.selected_model,
        selected_reasoning_effort: settings.selected_reasoning_effort,
        rate_limit: codex_rate_limit_snapshot().await,
        generation_adapter_available: codex_generation_available().await,
    }))
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartOAuthRequest {
    pub client_id: Option<String>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthStartView {
    pub flow_id: String,
    pub mode: String,
    pub status: String,
    pub auth_url: Option<String>,
    pub redirect_uri: Option<String>,
    pub user_code: Option<String>,
    pub verification_uri: Option<String>,
    pub expires_in: Option<u64>,
    pub interval: Option<u64>,
    pub disabled_reason: Option<String>,
}

#[tauri::command]
pub async fn start_github_oauth(
    request: StartOAuthRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<OAuthStartView> {
    let client_id = request
        .client_id
        .or_else(|| env::var("REVIEWDESK_GITHUB_CLIENT_ID").ok())
        .ok_or_else(|| {
            CommandError::new(
                "github_client_id_missing",
                "Set REVIEWDESK_GITHUB_CLIENT_ID to start GitHub OAuth.",
            )
        })?;
    let mode = request.mode.unwrap_or_else(|| "browser".to_string());
    if mode == "device" {
        return start_github_device_oauth(client_id, state).await;
    }
    if mode != "browser" {
        return Err(CommandError::new(
            "invalid_github_oauth_mode",
            "GitHub OAuth mode must be browser or device.",
        ));
    }

    let broker_base = env::var("REVIEWDESK_OAUTH_BROKER_URL").map_err(|_| {
        CommandError::new(
            "github_broker_missing",
            "Set REVIEWDESK_OAUTH_BROKER_URL to start broker-backed GitHub browser OAuth.",
        )
    })?;
    start_github_browser_oauth(client_id, broker_base, state).await
}

async fn start_github_device_oauth(
    client_id: String,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<OAuthStartView> {
    let client = GitHubClient::new("");
    let flow = client
        .start_device_flow(&client_id, GITHUB_OAUTH_SCOPES)
        .await?;

    *state
        .github_flow
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "GitHub OAuth state lock failed"))? =
        Some(GitHubOAuthFlow::Device {
            client_id,
            device_code: flow.device_code,
        });

    Ok(OAuthStartView {
        flow_id: GITHUB_FLOW_ID.to_string(),
        mode: "device".to_string(),
        status: "pending".to_string(),
        auth_url: None,
        redirect_uri: None,
        user_code: Some(flow.user_code),
        verification_uri: Some(flow.verification_uri),
        expires_in: Some(flow.expires_in),
        interval: Some(flow.interval),
        disabled_reason: None,
    })
}

async fn start_github_browser_oauth(
    client_id: String,
    broker_base: String,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<OAuthStartView> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| CommandError::new("loopback_port_unavailable", error.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|error| CommandError::new("loopback_port_unavailable", error.to_string()))?
        .port();
    let local_redirect_uri = format!("http://127.0.0.1:{port}{GITHUB_OAUTH_CALLBACK_PATH}");
    let broker_redirect_uri = format!(
        "{}/auth/github/callback",
        broker_base.trim_end_matches('/')
    );
    let state_value = random_hex(32)?;
    let code_verifier = random_hex(48)?;
    let code_challenge = pkce_challenge_s256(&code_verifier);
    let auth_url = build_github_authorize_url(
        &client_id,
        &broker_redirect_uri,
        GITHUB_OAUTH_SCOPES,
        &state_value,
        &code_challenge,
    )?;
    let flow_id = format!("github-browser-{}", random_hex(8)?);
    let github_flow = state.github_flow.clone();

    *github_flow
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "GitHub OAuth state lock failed"))? =
        Some(GitHubOAuthFlow::Browser {
            flow_id: flow_id.clone(),
            status: GitHubBrowserOAuthStatus::CallbackWaiting,
            disabled_reason: None,
        });

    tokio::spawn(run_github_browser_oauth_callback(
        listener,
        github_flow,
        flow_id.clone(),
        broker_base,
        local_redirect_uri.clone(),
        state_value,
        code_verifier,
    ));

    Ok(OAuthStartView {
        flow_id,
        mode: "browser".to_string(),
        status: "callback_waiting".to_string(),
        auth_url: Some(auth_url.to_string()),
        redirect_uri: Some(local_redirect_uri),
        user_code: None,
        verification_uri: None,
        expires_in: Some(GITHUB_OAUTH_EXPIRES_IN_SECONDS),
        interval: Some(2),
        disabled_reason: None,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct PollOAuthRequest {
    pub flow_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthPollView {
    pub flow_id: Option<String>,
    pub mode: Option<String>,
    pub status: String,
    pub account_login: Option<String>,
    pub disabled_reason: Option<String>,
}

#[tauri::command]
pub async fn poll_github_oauth(
    request: PollOAuthRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<OAuthPollView> {
    let flow = state
        .github_flow
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "GitHub OAuth state lock failed"))?
        .clone()
        .ok_or_else(|| CommandError::new("github_flow_missing", "Start GitHub OAuth first."))?;

    match flow {
        GitHubOAuthFlow::Device {
            client_id,
            device_code,
        } => {
            if request.flow_id != GITHUB_FLOW_ID {
                return Err(CommandError::new(
                    "unknown_oauth_flow",
                    "Unknown GitHub OAuth flow id.",
                ));
            }
            let client = GitHubClient::new("");
            match client.poll_device_token(&client_id, &device_code).await? {
                DevicePoll::Pending => Ok(OAuthPollView {
                    flow_id: Some(GITHUB_FLOW_ID.to_string()),
                    mode: Some("device".to_string()),
                    status: "pending".to_string(),
                    account_login: None,
                    disabled_reason: None,
                }),
                DevicePoll::SlowDown => Ok(OAuthPollView {
                    flow_id: Some(GITHUB_FLOW_ID.to_string()),
                    mode: Some("device".to_string()),
                    status: "slow_down".to_string(),
                    account_login: None,
                    disabled_reason: None,
                }),
                DevicePoll::Authorized(token) => {
                    let mut store = KeyringTokenStore::default();
                    store.save(TokenKind::GitHub, &token)?;
                    *state.github_flow.lock().map_err(|_| {
                        CommandError::new("state_lock_failed", "GitHub OAuth state lock failed")
                    })? = None;
                    Ok(OAuthPollView {
                        flow_id: Some(GITHUB_FLOW_ID.to_string()),
                        mode: Some("device".to_string()),
                        status: "authorized".to_string(),
                        account_login: None,
                        disabled_reason: None,
                    })
                }
                DevicePoll::Denied(reason) => Ok(OAuthPollView {
                    flow_id: Some(GITHUB_FLOW_ID.to_string()),
                    mode: Some("device".to_string()),
                    status: "denied".to_string(),
                    account_login: None,
                    disabled_reason: Some(reason),
                }),
            }
        }
        GitHubOAuthFlow::Browser {
            flow_id,
            status,
            disabled_reason,
        } => {
            if request.flow_id != flow_id {
                return Err(CommandError::new(
                    "unknown_oauth_flow",
                    "Unknown GitHub OAuth flow id.",
                ));
            }
            Ok(OAuthPollView {
                flow_id: Some(flow_id),
                mode: Some("browser".to_string()),
                status: status.as_str().to_string(),
                account_login: None,
                disabled_reason,
            })
        }
    }
}

#[tauri::command]
pub fn cancel_github_oauth(
    request: PollOAuthRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<OAuthPollView> {
    let mut flow = state
        .github_flow
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "GitHub OAuth state lock failed"))?;
    let matches_flow = match flow.as_ref() {
        Some(GitHubOAuthFlow::Device { .. }) => request.flow_id == GITHUB_FLOW_ID,
        Some(GitHubOAuthFlow::Browser { flow_id, .. }) => request.flow_id == *flow_id,
        None => false,
    };
    if !matches_flow {
        return Err(CommandError::new(
            "github_flow_missing",
            "No matching GitHub OAuth flow is pending.",
        ));
    }
    if let Some(GitHubOAuthFlow::Browser { flow_id, .. }) = flow.as_ref() {
        *flow = Some(GitHubOAuthFlow::Browser {
            flow_id: flow_id.clone(),
            status: GitHubBrowserOAuthStatus::Cancelled,
            disabled_reason: None,
        });
    } else {
        *flow = None;
    }
    Ok(OAuthPollView {
        flow_id: Some(request.flow_id),
        mode: None,
        status: "cancelled".to_string(),
        account_login: None,
        disabled_reason: None,
    })
}

#[tauri::command]
pub fn logout_github(state: tauri::State<'_, ReviewDeskTauriState>) -> CommandResult<OAuthPollView> {
    let mut store = KeyringTokenStore::default();
    store.delete(TokenKind::GitHub)?;
    *state
        .github_flow
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "GitHub OAuth state lock failed"))? =
        None;
    Ok(OAuthPollView {
        flow_id: None,
        mode: None,
        status: "missing".to_string(),
        account_login: None,
        disabled_reason: None,
    })
}

#[tauri::command]
pub async fn refresh_github_auth_status(
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<OAuthPollView> {
    let store = KeyringTokenStore::default();
    let status = github_connection_state(&store, &state).await?;
    Ok(OAuthPollView {
        flow_id: None,
        mode: None,
        status: github_auth_state_label(status).to_string(),
        account_login: None,
        disabled_reason: None,
    })
}

async fn github_connection_state(
    store: &KeyringTokenStore,
    state: &tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<reviewdesk::app_core::AuthConnectionState> {
    if github_browser_flow_is_authorized(state)? {
        return Ok(reviewdesk::app_core::AuthConnectionState::Connected);
    }
    let Some(token) = store.load(TokenKind::GitHub)? else {
        return Ok(reviewdesk::app_core::AuthConnectionState::Missing);
    };
    match GitHubClient::new(token).current_user().await {
        Ok(_) => Ok(reviewdesk::app_core::AuthConnectionState::Connected),
        Err(ReviewDeskError::GitHubApi { kind, .. }) => Ok(match kind {
            GitHubErrorKind::AuthRequired => reviewdesk::app_core::AuthConnectionState::ReauthRequired,
            GitHubErrorKind::ScopeMissing => reviewdesk::app_core::AuthConnectionState::ScopeMissing,
            GitHubErrorKind::SsoRequired => reviewdesk::app_core::AuthConnectionState::SsoRequired,
            GitHubErrorKind::RateLimited | GitHubErrorKind::SecondaryRateLimited | GitHubErrorKind::SearchRateLimited => {
                reviewdesk::app_core::AuthConnectionState::RateLimited
            }
            _ => reviewdesk::app_core::AuthConnectionState::Missing,
        }),
        Err(_) => Ok(reviewdesk::app_core::AuthConnectionState::Missing),
    }
}

fn github_auth_state_label(state: reviewdesk::app_core::AuthConnectionState) -> &'static str {
    match state {
        reviewdesk::app_core::AuthConnectionState::Connected => "connected",
        reviewdesk::app_core::AuthConnectionState::ScopeMissing => "github_scope_insufficient",
        reviewdesk::app_core::AuthConnectionState::SsoRequired => "github_sso_required",
        reviewdesk::app_core::AuthConnectionState::RateLimited => "github_rate_limited",
        reviewdesk::app_core::AuthConnectionState::ReauthRequired
        | reviewdesk::app_core::AuthConnectionState::Expired => "invalid_or_revoked",
        _ => "missing",
    }
}

fn github_browser_flow_is_authorized(
    state: &tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<bool> {
    Ok(matches!(
        state
            .github_flow
            .lock()
            .map_err(|_| CommandError::new("state_lock_failed", "GitHub OAuth state lock failed"))?
            .as_ref(),
        Some(GitHubOAuthFlow::Browser {
            status: GitHubBrowserOAuthStatus::Authorized,
            ..
        })
    ))
}

async fn run_github_browser_oauth_callback(
    listener: TcpListener,
    github_flow: Arc<Mutex<Option<GitHubOAuthFlow>>>,
    flow_id: String,
    broker_base: String,
    _local_redirect_uri: String,
    expected_state: String,
    code_verifier: String,
) {
    let result = timeout(
        Duration::from_secs(GITHUB_OAUTH_EXPIRES_IN_SECONDS),
        listener.accept(),
    )
    .await;

    let (mut stream, _) = match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            set_github_browser_flow_status(
                &github_flow,
                &flow_id,
                GitHubBrowserOAuthStatus::Failed,
                Some(error.to_string()),
            );
            return;
        }
        Err(_) => {
            set_github_browser_flow_status(
                &github_flow,
                &flow_id,
                GitHubBrowserOAuthStatus::Timeout,
                Some("loopback_callback_timeout".to_string()),
            );
            return;
        }
    };

    let mut buffer = vec![0_u8; 4096];
    let read = match stream.read(&mut buffer).await {
        Ok(read) => read,
        Err(error) => {
            set_github_browser_flow_status(
                &github_flow,
                &flow_id,
                GitHubBrowserOAuthStatus::Failed,
                Some(error.to_string()),
            );
            return;
        }
    };
    let request = String::from_utf8_lossy(&buffer[..read]);
    let callback = parse_loopback_callback(&request);

    let response_message = match callback {
        Ok(LoopbackCallback::Denied(reason)) => {
            set_github_browser_flow_status(
                &github_flow,
                &flow_id,
                GitHubBrowserOAuthStatus::Denied,
                Some(reason),
            );
            "ReviewDesk GitHub authorization was denied. You can close this window."
        }
        Ok(LoopbackCallback::Code { code, state }) => {
            if state != expected_state {
                set_github_browser_flow_status(
                    &github_flow,
                    &flow_id,
                    GitHubBrowserOAuthStatus::Failed,
                    Some("loopback_callback_state_mismatch".to_string()),
                );
                "ReviewDesk rejected this callback because the OAuth state did not match."
            } else if !github_browser_flow_is_active(&github_flow, &flow_id) {
                "ReviewDesk authorization was cancelled. You can close this window."
            } else {
                let client = GitHubClient::new("");
                match client
                    .redeem_broker_oauth_code(&broker_base, &code, &code_verifier)
                    .await
                {
                    Ok(token) => {
                        let save_result = KeyringTokenStore::default()
                            .save(TokenKind::GitHub, &token.access_token);
                        match save_result {
                            Ok(()) => {
                                set_github_browser_flow_status(
                                    &github_flow,
                                    &flow_id,
                                    GitHubBrowserOAuthStatus::Authorized,
                                    None,
                                );
                                "ReviewDesk GitHub authorization is complete. You can close this window."
                            }
                            Err(error) => {
                                set_github_browser_flow_status(
                                    &github_flow,
                                    &flow_id,
                                    GitHubBrowserOAuthStatus::Failed,
                                    Some(error.to_string()),
                                );
                                "ReviewDesk could not save the GitHub credential."
                            }
                        }
                    }
                    Err(error) => {
                        set_github_browser_flow_status(
                            &github_flow,
                            &flow_id,
                            GitHubBrowserOAuthStatus::Failed,
                            Some(error.to_string()),
                        );
                        "ReviewDesk could not finish GitHub authorization."
                    }
                }
            }
        }
        Err(reason) => {
            set_github_browser_flow_status(
                &github_flow,
                &flow_id,
                GitHubBrowserOAuthStatus::Failed,
                Some(reason),
            );
            "ReviewDesk rejected this OAuth callback."
        }
    };

    let _ = write_loopback_response(&mut stream, response_message).await;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LoopbackCallback {
    Code { code: String, state: String },
    Denied(String),
}

fn parse_loopback_callback(request: &str) -> Result<LoopbackCallback, String> {
    let first_line = request
        .lines()
        .next()
        .ok_or_else(|| "loopback_callback_invalid".to_string())?;
    let mut parts = first_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "loopback_callback_invalid".to_string())?;
    let target = parts
        .next()
        .ok_or_else(|| "loopback_callback_invalid".to_string())?;
    if method != "GET" {
        return Err("loopback_callback_method_invalid".to_string());
    }
    let url = url::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| "loopback_callback_invalid".to_string())?;
    if url.path() != GITHUB_OAUTH_CALLBACK_PATH {
        return Err("loopback_callback_origin_invalid".to_string());
    }
    let query = url
        .query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<std::collections::HashMap<_, _>>();
    if let Some(error) = query.get("error") {
        return Ok(LoopbackCallback::Denied(error.clone()));
    }
    let code = query
        .get("broker_code")
        .or_else(|| query.get("code"))
        .ok_or_else(|| "loopback_callback_broker_code_missing".to_string())?
        .clone();
    let state = query
        .get("state")
        .ok_or_else(|| "loopback_callback_state_missing".to_string())?
        .clone();
    Ok(LoopbackCallback::Code { code, state })
}

fn github_browser_flow_is_active(
    github_flow: &Arc<Mutex<Option<GitHubOAuthFlow>>>,
    expected_flow_id: &str,
) -> bool {
    github_flow
        .lock()
        .ok()
        .and_then(|flow| flow.clone())
        .map(|flow| match flow {
            GitHubOAuthFlow::Browser {
                flow_id, status, ..
            } => flow_id == expected_flow_id && status == GitHubBrowserOAuthStatus::CallbackWaiting,
            GitHubOAuthFlow::Device { .. } => false,
        })
        .unwrap_or(false)
}

fn set_github_browser_flow_status(
    github_flow: &Arc<Mutex<Option<GitHubOAuthFlow>>>,
    expected_flow_id: &str,
    status: GitHubBrowserOAuthStatus,
    disabled_reason: Option<String>,
) {
    if let Ok(mut flow) = github_flow.lock() {
        if let Some(GitHubOAuthFlow::Browser { flow_id, .. }) = flow.as_ref() {
            if flow_id == expected_flow_id {
                *flow = Some(GitHubOAuthFlow::Browser {
                    flow_id: flow_id.clone(),
                    status,
                    disabled_reason,
                });
            }
        }
    }
}

async fn write_loopback_response(
    stream: &mut tokio::net::TcpStream,
    message: &str,
) -> std::io::Result<()> {
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>ReviewDesk</title></head><body><p>{message}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await
}

fn random_hex(bytes: usize) -> CommandResult<String> {
    let mut random = vec![0_u8; bytes];
    getrandom::fill(&mut random)
        .map_err(|error| CommandError::new("secure_random_failed", error.to_string()))?;
    Ok(hex_string(&random))
}

fn hex_string(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(TABLE[(byte >> 4) as usize] as char);
        output.push(TABLE[(byte & 0x0f) as usize] as char);
    }
    output
}

#[tauri::command]
pub async fn get_ai_connection_status() -> AiConnectionStatusView {
    codex_ai_connection_status().await
}

#[tauri::command]
pub async fn get_codex_bridge_status() -> CodexBridgeStatusView {
    match CodexBridge::resolve_from_env() {
        Ok(bridge) => bridge.status().await,
        Err(error) => CodexBridgeStatusView::blocked(
            AiBlockedReason::CodexCliMissing,
            vec![sanitize_codex_diagnostics(&error.to_string())],
        ),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartCodexChatGptLoginRequest {
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexChatGptLoginStartView {
    pub login_id: String,
    pub status: String,
    pub mode: String,
    pub auth_url: Option<String>,
    pub verification_url: Option<String>,
    pub user_code: Option<String>,
    pub disabled_reason: Option<String>,
}

#[tauri::command]
pub async fn start_codex_chatgpt_login(
    request: StartCodexChatGptLoginRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<CodexChatGptLoginStartView> {
    let mode = request.mode.unwrap_or_else(|| "browser".to_string());
    if mode != "browser" && mode != "device" {
        return Err(CommandError::new(
            "invalid_codex_login_mode",
            "Codex ChatGPT login mode must be browser or device.",
        ));
    }

    let connection = codex_ai_connection_status().await;
    if connection.status == AiConnectionStatus::Connected {
        return Ok(CodexChatGptLoginStartView {
            login_id: "codex-chatgpt-already-connected".to_string(),
            status: "connected".to_string(),
            mode,
            auth_url: None,
            verification_url: None,
            user_code: None,
            disabled_reason: None,
        });
    }

    if connection.status == AiConnectionStatus::Unsupported {
        return Ok(CodexChatGptLoginStartView {
            login_id: "codex-chatgpt-unsupported".to_string(),
            status: "blocked".to_string(),
            mode,
            auth_url: None,
            verification_url: None,
            user_code: None,
            disabled_reason: Some(AiBlockedReason::CodexChatgptUnsupported.as_str().to_string()),
        });
    }

    let bridge = CodexBridge::resolve_from_env()
        .map_err(|_| CommandError::new("codex_cli_missing", "Codex CLI was not found on PATH."))?;
    let Some((login_id, auth_url, verification_url, user_code)) = bridge.start_login(&mode).await?
    else {
        return Ok(CodexChatGptLoginStartView {
            login_id: format!("codex-chatgpt-{}-login", mode),
            status: "blocked".to_string(),
            mode,
            auth_url: None,
            verification_url: None,
            user_code: None,
            disabled_reason: Some(AiBlockedReason::CodexAppServerUnavailable.as_str().to_string()),
        });
    };
    *state
        .codex_login
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "Codex login state lock failed"))? =
        Some(CodexLoginFlow {
            login_id: login_id.clone(),
        });

    if let Some(url) = auth_url.as_deref().or(verification_url.as_deref()) {
        validate_external_url(&url)
            .map_err(|_| CommandError::new("external_url_not_allowed", "Codex login URL is not allowlisted."))?;
    }

    Ok(CodexChatGptLoginStartView {
        login_id,
        status: "pending".to_string(),
        mode,
        auth_url,
        verification_url,
        user_code,
        disabled_reason: None,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelCodexChatGptLoginRequest {
    pub login_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PollCodexChatGptLoginRequest {
    pub login_id: String,
}

#[tauri::command]
pub async fn poll_codex_chatgpt_login(
    request: PollCodexChatGptLoginRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<OAuthPollView> {
    let matches_login = state
        .codex_login
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "Codex login state lock failed"))?
        .as_ref()
        .map(|flow| flow.login_id == request.login_id)
        .unwrap_or(false);
    if !matches_login {
        return Err(CommandError::new(
            "codex_login_missing",
            "No matching Codex ChatGPT login is pending.",
        ));
    }
    let connection = codex_ai_connection_status_with_refresh(true).await;
    if connection.status == AiConnectionStatus::Connected {
        *state.codex_login.lock().map_err(|_| {
            CommandError::new("state_lock_failed", "Codex login state lock failed")
        })? = None;
        Ok(OAuthPollView {
            flow_id: Some(request.login_id),
            mode: Some("codex_chatgpt".to_string()),
            status: "authorized".to_string(),
            account_login: connection
                .account
                .and_then(|account| account.display_label)
                .or(Some("Codex ChatGPT".to_string())),
            disabled_reason: None,
        })
    } else {
        Ok(OAuthPollView {
            flow_id: Some(request.login_id),
            mode: Some("codex_chatgpt".to_string()),
            status: "pending".to_string(),
            account_login: None,
            disabled_reason: connection.blocked_reason.map(|reason| reason.as_str().to_string()),
        })
    }
}

#[tauri::command]
pub fn cancel_codex_chatgpt_login(
    request: CancelCodexChatGptLoginRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<OAuthPollView> {
    let mut login = state
        .codex_login
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "Codex login state lock failed"))?;
    let matches_login = login
        .as_ref()
        .map(|flow| flow.login_id == request.login_id)
        .unwrap_or(false);
    if !matches_login {
        return Err(CommandError::new(
            "codex_login_missing",
            "No matching Codex ChatGPT login is pending.",
        ));
    }
    *login = None;
    Ok(OAuthPollView {
        flow_id: Some(request.login_id),
        mode: Some("codex_chatgpt".to_string()),
        status: "cancelled".to_string(),
        account_login: None,
        disabled_reason: None,
    })
}

#[tauri::command]
pub async fn read_codex_account() -> AiConnectionStatusView {
    codex_ai_connection_status().await
}

#[derive(Debug, Clone, Serialize)]
pub struct AiModelListView {
    pub status: String,
    pub models: Vec<AiModelView>,
    pub disabled_reason: Option<String>,
}

#[tauri::command]
pub async fn list_ai_models() -> AiModelListView {
    let connection = codex_ai_connection_status().await;
    let models = codex_ai_models_for(&connection).await;
    if models.is_empty() {
        AiModelListView {
            status: "blocked".to_string(),
            models,
            disabled_reason: Some(AiBlockedReason::ModelListUnavailable.as_str().to_string()),
        }
    } else {
        AiModelListView {
            status: "ready".to_string(),
            models,
            disabled_reason: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SelectAiModelRequest {
    pub model: String,
    pub reasoning_effort: String,
}

#[tauri::command]
pub async fn select_ai_model(
    request: SelectAiModelRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<AppStatusView> {
    let reasoning_effort = parse_reasoning_effort(&request.reasoning_effort)?;
    let connection = codex_ai_connection_status().await;
    let models = codex_ai_models_for(&connection).await;
    if !models.is_empty() {
        validate_ai_run_configuration(&models, &request.model, reasoning_effort)
            .map_err(|reason| CommandError::new(reason.as_str(), reason.as_str()))?;
    }

    *state
        .ai_settings
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "AI settings state lock failed"))? =
        AiLocalSettings {
            selected_model: request.model,
            selected_reasoning_effort: reasoning_effort,
        };
    get_app_status(state).await
}

#[tauri::command]
pub async fn read_codex_rate_limits() -> AiRateLimitSnapshot {
    codex_rate_limit_snapshot().await
}

#[tauri::command]
pub async fn logout_codex_chatgpt() -> CommandResult<AiConnectionStatusView> {
    if let Ok(bridge) = CodexBridge::resolve_from_env() {
        bridge.logout_account().await?;
    }
    Ok(codex_ai_connection_status().await)
}

#[tauri::command]
pub async fn refresh_ai_account_status() -> AiConnectionStatusView {
    codex_ai_connection_status_with_refresh(true).await
}

#[tauri::command]
pub fn start_chatgpt_oauth(_request: StartOAuthRequest) -> OAuthStartView {
    OAuthStartView {
        flow_id: "codex-chatgpt-managed-auth-required".to_string(),
        mode: "browser".to_string(),
        status: "blocked".to_string(),
        auth_url: None,
        redirect_uri: None,
        user_code: None,
        verification_uri: None,
        expires_in: None,
        interval: None,
        disabled_reason: Some(AiBlockedReason::CodexChatgptUnsupported.as_str().to_string()),
    }
}

#[tauri::command]
pub fn poll_chatgpt_oauth(_request: PollOAuthRequest) -> OAuthPollView {
    OAuthPollView {
        flow_id: None,
        mode: Some("codex_chatgpt".to_string()),
        status: "blocked".to_string(),
        account_login: None,
        disabled_reason: Some("Official ChatGPT OAuth integration required.".to_string()),
    }
}

#[tauri::command]
pub async fn list_repositories() -> CommandResult<Vec<Repository>> {
    let client = github_client_from_keychain()?;
    Ok(client.list_repositories().await?)
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoadReviewQueueRequest {
    pub repository: Option<String>,
}

#[tauri::command]
pub async fn load_review_queue(
    request: LoadReviewQueueRequest,
) -> CommandResult<Vec<PullRequestQueueItem>> {
    let client = github_client_from_keychain()?;
    let user = client.current_user().await?;
    let repo = request
        .repository
        .as_deref()
        .filter(|value| !value.is_empty() && *value != "all");
    Ok(client.review_queue(&user.login, repo).await?)
}

#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

#[tauri::command]
pub async fn collect_pr_context(
    request: PullRequestRequest,
) -> CommandResult<PullRequestContextView> {
    let client = github_client_from_keychain()?;
    Ok(client
        .collect_pull_request_context_view(&request.owner, &request.repo, request.number)
        .await?)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrivateDiffConsentRequest {
    pub repository: String,
    pub provider: String,
    pub model: String,
    pub transmitted_scope: String,
    pub accepted: bool,
}

#[tauri::command]
pub fn set_private_diff_consent(request: PrivateDiffConsentRequest) -> CommandResult<String> {
    let store = local_store()?;
    let consent = PrivateDiffConsent {
        repository_private: true,
        accepted: request.accepted,
        provider: request.provider,
        model: request.model,
        transmitted_scope: request.transmitted_scope,
    };
    let path = store.save_json(
        &format!("state/private-diff-consent-{}.json", safe_slug(&request.repository)),
        &consent,
    )?;
    Ok(path.display().to_string())
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateReviewDraftRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub files: Vec<ChangedFile>,
    pub model: String,
    pub reasoning_depth: String,
    pub reasoning_effort: Option<String>,
    pub review_language: Option<Locale>,
    pub head_sha: Option<String>,
    pub private_diff_consent_required: bool,
    pub private_diff_consent_accepted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedDraftView {
    pub status: String,
    pub body: String,
    pub report_path: Option<String>,
    pub disabled_reason: Option<String>,
    pub blocked_reason: Option<String>,
    pub run: Option<AgentRunRecordView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRunStateView {
    pub run: AgentRunRecordView,
    pub draft_body: Option<String>,
    pub report_path: Option<String>,
    pub disabled_reason: Option<String>,
}

#[tauri::command]
pub async fn start_agent_run(
    request: GenerateReviewDraftRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<GeneratedDraftView> {
    run_agent_review(request, state).await
}

#[tauri::command]
pub fn read_agent_run(
    request: ReadAgentRunRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<AgentRunStateView> {
    state
        .agent_runs
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "Agent run state lock failed"))?
        .get(&request.run_id)
        .cloned()
        .ok_or_else(|| CommandError::new("agent_run_not_found", "Agent run was not found."))
}

#[tauri::command]
pub fn cancel_agent_run(
    request: ReadAgentRunRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<AgentRunStateView> {
    let mut runs = state
        .agent_runs
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "Agent run state lock failed"))?;
    let run = runs
        .get_mut(&request.run_id)
        .ok_or_else(|| CommandError::new("agent_run_not_found", "Agent run was not found."))?;
    if run.run.status == "queued" || run.run.status == "running" {
        run.run.status = "cancelled".to_string();
        run.run.blocked_reason = Some(AiBlockedReason::CodexGenerationInterrupted);
        run.run.completed_at = Some(chrono::Utc::now().to_rfc3339());
        run.disabled_reason = Some(AiBlockedReason::CodexGenerationInterrupted.as_str().to_string());
    }
    Ok(run.clone())
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadAgentRunRequest {
    pub run_id: String,
}

#[tauri::command]
pub async fn generate_review_draft(
    request: GenerateReviewDraftRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<GeneratedDraftView> {
    run_agent_review(request, state).await
}

async fn run_agent_review(
    request: GenerateReviewDraftRequest,
    state: tauri::State<'_, ReviewDeskTauriState>,
) -> CommandResult<GeneratedDraftView> {
    let review_language = request.review_language.unwrap_or(Locale::En);
    let reasoning_effort = parse_reasoning_effort(
        request
            .reasoning_effort
            .as_deref()
            .unwrap_or(request.reasoning_depth.as_str()),
    )?;
    let selected_files = request
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    let head_sha = request.head_sha.clone().unwrap_or_default();
    let run_id =
        reviewdesk::review::new_analysis_run_id(&request.owner, &request.repo, request.number);
    let diff_hash = reviewdesk::review::stable_changed_files_hash(&request.files);
    let rate_limit = codex_rate_limit_snapshot().await;
    let mut run = AgentRunRecordView {
        run_id,
        pull_request_id: format!("{}/{}#{}", request.owner, request.repo, request.number),
        repo_full_name: format!("{}/{}", request.owner, request.repo),
        pull_number: request.number,
        head_sha,
        diff_hash,
        selected_files,
        provider: "codex_chatgpt".to_string(),
        auth_mode: "chatgpt".to_string(),
        plan_type: None,
        model_id: request.model.clone(),
        reasoning_effort,
        review_language,
        prompt_version: "reviewdesk-0.0.9".to_string(),
        private_diff_consent_snapshot: request.private_diff_consent_accepted,
        rate_limit_snapshot: Some(rate_limit.clone()),
        status: "blocked".to_string(),
        blocked_reason: None,
        started_at: chrono::Utc::now().to_rfc3339(),
        completed_at: None,
    };

    if request.private_diff_consent_required && !request.private_diff_consent_accepted {
        return blocked_generated_draft(state, run, AiBlockedReason::PrivateDiffConsentRequired);
    }

    let connection = codex_ai_connection_status().await;
    run.plan_type = connection.plan_type.clone();
    if connection.status != AiConnectionStatus::Connected {
        let reason = connection
            .blocked_reason
            .unwrap_or(AiBlockedReason::CodexChatgptAuthRequired);
        return blocked_generated_draft(state, run, reason);
    }

    let models = codex_ai_models_for(&connection).await;
    if models.is_empty() {
        return blocked_generated_draft(state, run, AiBlockedReason::ModelListUnavailable);
    }
    if let Err(reason) =
        validate_ai_run_configuration(&models, &request.model, reasoning_effort)
    {
        return blocked_generated_draft(state, run, reason);
    }
    if let Some(reason) = rate_limit.blocked_reason {
        return blocked_generated_draft(state, run, reason);
    }
    if !codex_generation_available().await {
        return blocked_generated_draft(state, run, AiBlockedReason::GenerationAdapterUnavailable);
    }

    let input = build_review_input(
        format!("{}/{}#{}", request.owner, request.repo, request.number),
        request.files.clone(),
        vec![
            format!("model={}", request.model),
            format!("reasoning_effort={}", reasoning_effort),
            format!("review_language={:?}", review_language),
        ],
    );
    run.status = "running".to_string();
    remember_agent_run(&state, &run, None, None, None)?;

    let (draft_body, markdown, disabled_reason) = if mock_ai_enabled() {
        let pipeline = ReviewPipeline::default();
        let provider = reviewdesk::ai::MockAiProvider::default();
        let result = pipeline.generate_or_block(&provider, input).await?;
        (result.draft.body, result.markdown, result.run.error)
    } else {
        let bridge = CodexBridge::resolve_from_env()
            .map_err(|_| CommandError::new("codex_cli_missing", "Codex CLI was not found on PATH."))?;
        let prompt = build_codex_review_prompt(
            &request.owner,
            &request.repo,
            request.number,
            &request.model,
            reasoning_effort,
            review_language,
            &request.files,
        );
        match bridge
            .generate_with_exec_json(&prompt, &request.model, reasoning_effort, None)
            .await
        {
            Ok(draft) => {
                let sanitized = reviewdesk::security::SecretMasker::default().mask(&draft).text;
                let markdown = format!(
                    "# ReviewDesk Codex Report\n\nPR: {}/{}#{}\n\n## Draft\n\n{}\n",
                    request.owner, request.repo, request.number, sanitized
                );
                (sanitized, markdown, None)
            }
            Err(reason) => {
                run.status = "failed".to_string();
                run.blocked_reason = Some(reason);
                run.completed_at = Some(chrono::Utc::now().to_rfc3339());
                remember_agent_run(
                    &state,
                    &run,
                    None,
                    None,
                    Some(reason.as_str().to_string()),
                )?;
                return Ok(GeneratedDraftView {
                    status: "failed".to_string(),
                    body: String::new(),
                    report_path: None,
                    disabled_reason: Some(reason.as_str().to_string()),
                    blocked_reason: Some(reason.as_str().to_string()),
                    run: Some(run),
                });
            }
        }
    };
    let store = local_store()?;
    run.status = "draft_ready".to_string();
    run.completed_at = Some(chrono::Utc::now().to_rfc3339());
    let run_path = store.save_json(
        &format!("runs/{}.json", safe_slug(&run.run_id)),
        &run,
    )?;
    let report_path = store.save_text(
        &format!(
            "reviews/{}-{}-{}.md",
            safe_slug(&request.owner),
            safe_slug(&request.repo),
            request.number
        ),
        &markdown,
    )?;
    remember_agent_run(
        &state,
        &run,
        Some(draft_body.clone()),
        Some(format!("{}\n{}", report_path.display(), run_path.display())),
        disabled_reason.clone(),
    )?;

    Ok(GeneratedDraftView {
        status: "draft_ready".to_string(),
        body: draft_body,
        report_path: Some(format!(
            "{}\n{}",
            report_path.display(),
            run_path.display()
        )),
        disabled_reason,
        blocked_reason: None,
        run: Some(run),
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDraftFromRunRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub source_run_ids: Vec<String>,
    pub base_head_sha: String,
    pub base_diff_hash: String,
    pub body: String,
    pub event: ReviewEvent,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveReviewDraftRequest {
    pub draft: ReviewDraft,
    pub mark_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadReviewDraftRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub draft_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListReviewDraftsRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarkActiveDraftRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub draft_id: String,
}

#[tauri::command]
pub fn create_draft_from_run(request: CreateDraftFromRunRequest) -> CommandResult<ReviewDraft> {
    let store = workspace_store()?;
    let key = PrWorkspaceKey::new(&request.owner, &request.repo, request.number)?;
    let now = chrono::Utc::now().to_rfc3339();
    let draft = ReviewDraft {
        draft_id: new_review_draft_id()?,
        owner: request.owner,
        repo: request.repo,
        number: request.number,
        source_run_ids: request.source_run_ids,
        base_head_sha: request.base_head_sha,
        base_diff_hash: request.base_diff_hash,
        verdict: request.event,
        body: request.body,
        inline_comments: vec![],
        user_edited: false,
        stale: false,
        created_at: now.clone(),
        updated_at: now,
    };

    store.save_draft(&key, &draft)?;
    store.mark_active_draft(&key, &draft.draft_id)?;
    Ok(draft)
}

#[tauri::command]
pub fn save_review_draft(request: SaveReviewDraftRequest) -> CommandResult<ReviewDraft> {
    let store = workspace_store()?;
    let key = PrWorkspaceKey::new(&request.draft.owner, &request.draft.repo, request.draft.number)?;
    store.save_draft(&key, &request.draft)?;
    if request.mark_active {
        store.mark_active_draft(&key, &request.draft.draft_id)?;
    }
    Ok(request.draft)
}

#[tauri::command]
pub fn read_review_draft(request: ReadReviewDraftRequest) -> CommandResult<ReviewDraft> {
    let store = workspace_store()?;
    let key = PrWorkspaceKey::new(request.owner, request.repo, request.number)?;
    Ok(store.read_draft(&key, &request.draft_id)?)
}

#[tauri::command]
pub fn list_review_drafts(request: ListReviewDraftsRequest) -> CommandResult<Vec<ReviewDraft>> {
    let store = workspace_store()?;
    let key = PrWorkspaceKey::new(request.owner, request.repo, request.number)?;
    Ok(store.list_drafts(&key)?)
}

#[tauri::command]
pub fn mark_active_draft(request: MarkActiveDraftRequest) -> CommandResult<Option<String>> {
    let store = workspace_store()?;
    let key = PrWorkspaceKey::new(request.owner, request.repo, request.number)?;
    store.mark_active_draft(&key, &request.draft_id)?;
    Ok(store.read_active_draft_id(&key)?)
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveDraftRequest {
    pub draft_id: String,
    pub body: String,
}

#[tauri::command]
pub fn save_draft(request: SaveDraftRequest) -> CommandResult<String> {
    let store = local_store()?;
    let path = store.save_text(
        &format!("drafts/{}.md", safe_slug(&request.draft_id)),
        &request.body,
    )?;
    Ok(path.display().to_string())
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrepareSubmitReviewRequest {
    pub payload: ReviewPublishPayload,
    pub github_connected: bool,
    pub write_scope_valid: bool,
    pub sso_required: bool,
    pub pr_open: bool,
    pub pr_merged: bool,
    pub current_head_sha: String,
    pub current_diff_hash: String,
}

#[tauri::command]
pub fn prepare_submit_review(request: PrepareSubmitReviewRequest) -> PreparedReviewPublishView {
    prepare_review_publish(
        request.payload,
        request.github_connected,
        request.write_scope_valid,
        request.sso_required,
        request.pr_open,
        request.pr_merged,
        &request.current_head_sha,
        &request.current_diff_hash,
    )
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidateInlineCommentsRequest {
    pub comments: Vec<InlineCommentDraft>,
    pub files: Vec<ChangedFile>,
    pub diff_hash: String,
}

#[tauri::command]
pub fn validate_inline_comments(
    request: ValidateInlineCommentsRequest,
) -> CommandResult<Vec<InlineCommentDraft>> {
    Ok(validate_inline_comments_core(
        &request.comments,
        &request.files,
        &request.diff_hash,
    )?)
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfirmSubmitReviewRequest {
    pub payload: ReviewPublishPayload,
    pub confirmation_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmittedReviewView {
    pub id: u64,
    pub url: String,
}

#[tauri::command]
pub async fn confirm_submit_review(
    request: ConfirmSubmitReviewRequest,
) -> CommandResult<SubmittedReviewView> {
    let store = workspace_store()?;
    let key = PrWorkspaceKey::new(
        request.payload.owner.clone(),
        request.payload.repo.clone(),
        request.payload.number,
    )?;

    let client = match github_client_from_keychain() {
        Ok(client) => client,
        Err(error) => {
            record_failed_publish_attempt(
                &store,
                &key,
                &request.payload,
                &request.confirmation_id,
                &error,
            );
            return Err(error);
        }
    };
    let snapshot = client
        .pull_request_snapshot(
            &request.payload.owner,
            &request.payload.repo,
            request.payload.number,
        )
        .await
        .map_err(|error| {
            record_publish_error(&store, &key, &request.payload, &request.confirmation_id, error)
        })?;
    let pull_files = client
        .list_pull_files(
            &request.payload.owner,
            &request.payload.repo,
            request.payload.number,
        )
        .await
        .map_err(|error| {
            record_publish_error(&store, &key, &request.payload, &request.confirmation_id, error)
        })?;
    let changed_files = pull_files
        .into_iter()
        .map(|file| ChangedFile {
            path: file.filename,
            patch: file.patch,
        })
        .collect::<Vec<_>>();
    let current_diff_hash = stable_changed_files_hash(&changed_files);
    let preflight = prepare_review_publish(
        request.payload.clone(),
        true,
        true,
        false,
        snapshot.state == "open",
        snapshot.merged,
        &snapshot.head_sha,
        &current_diff_hash,
    );
    if preflight.status == PreparedReviewPublishStatus::Blocked {
        let code = preflight
            .blocked_reasons
            .first()
            .cloned()
            .unwrap_or_else(|| "publish_blocked".to_string());
        let error = CommandError::new(code, "Submit preflight is blocked.");
        record_failed_publish_attempt(
            &store,
            &key,
            &request.payload,
            &request.confirmation_id,
            &error,
        );
        return Err(error);
    }
    if preflight.confirmation_id.as_deref() != Some(request.confirmation_id.as_str()) {
        let error = CommandError::new(
            "confirmation_mismatch",
            "Submit preflight must be prepared again for the current draft, head SHA, and diff.",
        );
        record_failed_publish_attempt(
            &store,
            &key,
            &request.payload,
            &request.confirmation_id,
            &error,
        );
        return Err(error);
    }

    let comments = request
        .payload
        .inline_comments
        .iter()
        .filter(|comment| {
            comment.selected_for_publish
                && !comment.dismissed
                && comment.mapping_status == InlineMappingStatus::Valid
                && !comment.body.trim().is_empty()
        })
        .map(|comment| ReviewSubmitComment {
            path: comment.path.clone(),
            side: comment.side.clone(),
            line: comment.line,
            start_line: comment.start_line,
            start_side: comment.start_side.clone(),
            body: comment.body.clone(),
        })
        .collect::<Vec<_>>();

    let submit_request = ReviewSubmitRequest {
        commit_id: snapshot.head_sha.clone(),
        event: request.payload.event,
        body: request.payload.body.clone(),
        comments,
    };

    let response: SubmittedReviewResponse = match client
        .submit_review(
            &request.payload.owner,
            &request.payload.repo,
            request.payload.number,
            &submit_request,
        )
        .await
    {
        Ok(response) => response,
        Err(error) => {
            let error = record_publish_error(
                &store,
                &key,
                &request.payload,
                &request.confirmation_id,
                error,
            );
            return Err(error);
        }
    };

    let now = chrono::Utc::now().to_rfc3339();
    save_publish_attempt(
        &store,
        &key,
        PublishAttempt {
            attempt_id: new_publish_attempt_id()?,
            draft_id: None,
            confirmation_id: request.confirmation_id.clone(),
            payload: request.payload.clone(),
            github_account: None,
            status: "succeeded".to_string(),
            github_review_id: Some(response.id),
            error_code: None,
            error_message: None,
            created_at: now.clone(),
            completed_at: Some(now),
        },
    )?;

    Ok(SubmittedReviewView {
        id: response.id,
        url: format!(
            "https://github.com/{}/{}/pull/{}#pullrequestreview-{}",
            request.payload.owner, request.payload.repo, request.payload.number, response.id
        ),
    })
}

fn record_publish_error(
    store: &WorkspaceStore,
    key: &PrWorkspaceKey,
    payload: &ReviewPublishPayload,
    confirmation_id: &str,
    error: ReviewDeskError,
) -> CommandError {
    let command_error = CommandError::from(error);
    record_failed_publish_attempt(store, key, payload, confirmation_id, &command_error);
    command_error
}

fn record_failed_publish_attempt(
    store: &WorkspaceStore,
    key: &PrWorkspaceKey,
    payload: &ReviewPublishPayload,
    confirmation_id: &str,
    error: &CommandError,
) {
    let Ok(attempt_id) = new_publish_attempt_id() else {
        return;
    };
    let now = chrono::Utc::now().to_rfc3339();
    let attempt = PublishAttempt {
        attempt_id,
        draft_id: None,
        confirmation_id: confirmation_id.to_string(),
        payload: payload.clone(),
        github_account: None,
        status: "failed".to_string(),
        github_review_id: None,
        error_code: Some(error.code.clone()),
        error_message: Some(error.message.clone()),
        created_at: now.clone(),
        completed_at: Some(now),
    };
    let _ = store.save_publish_attempt(key, &attempt);
}

fn save_publish_attempt(
    store: &WorkspaceStore,
    key: &PrWorkspaceKey,
    attempt: PublishAttempt,
) -> CommandResult<()> {
    store.save_publish_attempt(key, &attempt)?;
    Ok(())
}

fn new_publish_attempt_id() -> CommandResult<String> {
    let now = chrono::Utc::now();
    let timestamp = now
        .timestamp_nanos_opt()
        .map_or_else(|| now.timestamp_micros() * 1_000, |value| value);
    Ok(format!("publish-{timestamp}-{}", random_hex(8)?))
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenExternalUrlRequest {
    pub url: String,
}

#[tauri::command]
pub fn open_external_url(request: OpenExternalUrlRequest) -> CommandResult<()> {
    let url = validate_external_url(&request.url)
        .map_err(|_| CommandError::new("external_url_not_allowed", "Only HTTPS GitHub and ChatGPT URLs can be opened."))?;
    webbrowser::open(url.as_str())
        .map_err(|error| CommandError::new("open_external_url_failed", error.to_string()))?;
    Ok(())
}

async fn codex_ai_connection_status() -> AiConnectionStatusView {
    codex_ai_connection_status_with_refresh(false).await
}

async fn codex_ai_connection_status_with_refresh(refresh_token: bool) -> AiConnectionStatusView {
    let bridge = match CodexBridge::resolve_from_env() {
        Ok(bridge) => bridge,
        Err(_) => {
            return AiConnectionStatusView {
                provider: "codex_chatgpt".to_string(),
                status: AiConnectionStatus::Unsupported,
                auth_mode: None,
                account: None,
                plan_type: None,
                last_refreshed_at: Some(chrono::Utc::now().to_rfc3339()),
                blocked_reason: Some(AiBlockedReason::CodexCliMissing),
            };
        }
    };
    match bridge.read_account(refresh_token).await {
        Ok(mut status) => {
            status.last_refreshed_at = Some(chrono::Utc::now().to_rfc3339());
            status
        }
        Err(_) => AiConnectionStatusView {
            provider: "codex_chatgpt".to_string(),
            status: AiConnectionStatus::Unsupported,
            auth_mode: None,
            account: None,
            plan_type: None,
            last_refreshed_at: Some(chrono::Utc::now().to_rfc3339()),
            blocked_reason: Some(AiBlockedReason::CodexAppServerUnavailable),
        },
    }
}

async fn codex_ai_models_for(connection: &AiConnectionStatusView) -> Vec<AiModelView> {
    if connection.status != AiConnectionStatus::Connected {
        return Vec::new();
    }
    match CodexBridge::resolve_from_env() {
        Ok(bridge) => bridge.list_models().await.unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

async fn codex_rate_limit_snapshot() -> AiRateLimitSnapshot {
    if let Ok(bridge) = CodexBridge::resolve_from_env() {
        return bridge.read_rate_limits().await;
    }
    match env::var("REVIEWDESK_CODEX_RATE_LIMIT_STATUS").ok().as_deref() {
        Some("rate_limited") => AiRateLimitSnapshot {
            status: AiRateLimitStatus::RateLimited,
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
            resets_at: env::var("REVIEWDESK_CODEX_RATE_LIMIT_RESETS_AT").ok(),
            blocked_reason: Some(AiBlockedReason::CodexChatgptRateLimited),
        },
        Some("credits_depleted") => AiRateLimitSnapshot {
            status: AiRateLimitStatus::CreditsDepleted,
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
            resets_at: None,
            blocked_reason: Some(AiBlockedReason::CodexChatgptCreditsDepleted),
        },
        Some("ok") => AiRateLimitSnapshot {
            status: AiRateLimitStatus::Ok,
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
            resets_at: None,
            blocked_reason: None,
        },
        _ => AiRateLimitSnapshot::unknown(),
    }
}

async fn codex_generation_available() -> bool {
    if env::var("REVIEWDESK_ENABLE_MOCK_AI")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return true;
    }
    match CodexBridge::resolve_from_env() {
        Ok(bridge) => bridge.status().await.exec_available,
        Err(_) => false,
    }
}

fn parse_reasoning_effort(value: &str) -> CommandResult<ReasoningEffort> {
    match value {
        "minimal" => Ok(ReasoningEffort::Minimal),
        "low" | "fast" => Ok(ReasoningEffort::Low),
        "medium" | "balanced" => Ok(ReasoningEffort::Medium),
        "high" | "deep" => Ok(ReasoningEffort::High),
        "xhigh" => Ok(ReasoningEffort::Xhigh),
        other => Err(CommandError::new(
            "reasoning_effort_unavailable",
            format!("Unsupported reasoning effort: {other}"),
        )),
    }
}

fn blocked_generated_draft(
    state: tauri::State<'_, ReviewDeskTauriState>,
    mut run: AgentRunRecordView,
    reason: AiBlockedReason,
) -> CommandResult<GeneratedDraftView> {
    run.status = "blocked".to_string();
    run.blocked_reason = Some(reason);
    run.completed_at = Some(chrono::Utc::now().to_rfc3339());
    let report_path = local_store()
        .and_then(|store| {
            store
                .save_json(&format!("runs/{}.json", safe_slug(&run.run_id)), &run)
                .map_err(CommandError::from)
        })
        .ok()
        .map(|path| path.display().to_string());
    remember_agent_run(
        &state,
        &run,
        None,
        report_path.clone(),
        Some(reason.as_str().to_string()),
    )?;

    Ok(GeneratedDraftView {
        status: "blocked".to_string(),
        body: String::new(),
        report_path,
        disabled_reason: Some(reason.as_str().to_string()),
        blocked_reason: Some(reason.as_str().to_string()),
        run: Some(run),
    })
}

fn remember_agent_run(
    state: &tauri::State<'_, ReviewDeskTauriState>,
    run: &AgentRunRecordView,
    draft_body: Option<String>,
    report_path: Option<String>,
    disabled_reason: Option<String>,
) -> CommandResult<()> {
    state
        .agent_runs
        .lock()
        .map_err(|_| CommandError::new("state_lock_failed", "Agent run state lock failed"))?
        .insert(
            run.run_id.clone(),
            AgentRunStateView {
                run: run.clone(),
                draft_body,
                report_path,
                disabled_reason,
            },
        );
    Ok(())
}

fn mock_ai_enabled() -> bool {
    env::var("REVIEWDESK_ENABLE_MOCK_AI")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn build_codex_review_prompt(
    owner: &str,
    repo: &str,
    number: u64,
    model: &str,
    reasoning_effort: ReasoningEffort,
    review_language: Locale,
    files: &[ChangedFile],
) -> String {
    let masker = reviewdesk::security::SecretMasker::default();
    let mut prompt = format!(
        "You are ReviewDesk. Create a concise draft code review for {owner}/{repo}#{number}.\n\
         Requirements:\n\
         - Return only the review draft body.\n\
         - Use {:?} for the review language.\n\
         - Be polite, specific, and evidence-based.\n\
         - Prefer questions when uncertain.\n\
         - Do not invent findings outside the provided diff.\n\
         - Model: {model}, reasoning effort: {reasoning_effort}.\n\n\
         Changed files:\n",
        review_language
    );
    for file in files {
        prompt.push_str(&format!("\n## {}\n", file.path));
        match &file.patch {
            Some(patch) => {
                let masked = masker.mask(patch).text;
                prompt.push_str("```diff\n");
                prompt.push_str(&masked);
                prompt.push_str("\n```\n");
            }
            None => prompt.push_str("No textual patch available.\n"),
        }
    }
    prompt
}

fn github_client_from_keychain() -> CommandResult<GitHubClient> {
    let token = KeyringTokenStore::default()
        .load(TokenKind::GitHub)?
        .ok_or_else(|| CommandError::new("github_auth_required", "Connect GitHub first."))?;
    Ok(GitHubClient::new(token))
}

fn local_store() -> CommandResult<LocalStore> {
    let cwd = env::current_dir()
        .map_err(|error| CommandError::new("current_dir_failed", error.to_string()))?;
    Ok(LocalStore::init(cwd)?)
}

fn workspace_store() -> CommandResult<WorkspaceStore> {
    let cwd = env::current_dir()
        .map_err(|error| CommandError::new("current_dir_failed", error.to_string()))?;
    Ok(WorkspaceStore::init(cwd)?)
}

fn new_review_draft_id() -> CommandResult<String> {
    let now = chrono::Utc::now();
    let timestamp = now
        .timestamp_nanos_opt()
        .map_or_else(|| now.timestamp_micros() * 1_000, |value| value);
    Ok(format!("draft-{timestamp}-{}", random_hex(8)?))
}

fn safe_slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn github_error_code(kind: GitHubErrorKind) -> &'static str {
    match kind {
        GitHubErrorKind::AuthRequired => "github_auth_required",
        GitHubErrorKind::ScopeMissing => "github_scope_missing",
        GitHubErrorKind::SsoRequired => "github_sso_required",
        GitHubErrorKind::RateLimited => "github_rate_limited",
        GitHubErrorKind::SecondaryRateLimited => "github_secondary_rate_limited",
        GitHubErrorKind::SearchRateLimited => "github_search_rate_limited",
        GitHubErrorKind::NotFound => "github_not_found",
        GitHubErrorKind::Network => "github_network",
        GitHubErrorKind::Unknown => "github_unknown",
    }
}

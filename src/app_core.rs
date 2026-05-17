use crate::domain::ReviewEvent;
use crate::domain::{Result, ReviewDeskError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthConnectionState {
    Connected,
    Missing,
    Expired,
    ReauthRequired,
    ScopeMissing,
    SsoRequired,
    Unsupported,
    ModelUnavailable,
    RateLimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    Ko,
}

impl Locale {
    pub fn from_os_locale(value: &str) -> Self {
        if value.to_ascii_lowercase().starts_with("ko") {
            Self::Ko
        } else {
            Self::En
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguagePreferences {
    pub ui_locale: Locale,
    pub review_locale: Locale,
    pub repo_review_locale_overrides: BTreeMap<String, Locale>,
}

impl LanguagePreferences {
    pub fn for_os_locale(value: &str) -> Self {
        let locale = Locale::from_os_locale(value);
        Self {
            ui_locale: locale,
            review_locale: locale,
            repo_review_locale_overrides: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiConnectionStatus {
    Missing,
    Connecting,
    BrowserOpened,
    DeviceCodeWaiting,
    Connected,
    Expired,
    ReauthRequired,
    RefreshFailed,
    Unsupported,
    RateLimited,
}

impl AiConnectionStatus {
    fn legacy_auth_state(self) -> AuthConnectionState {
        match self {
            Self::Connected => AuthConnectionState::Connected,
            Self::Expired => AuthConnectionState::Expired,
            Self::ReauthRequired | Self::RefreshFailed => AuthConnectionState::ReauthRequired,
            Self::Unsupported => AuthConnectionState::Unsupported,
            Self::RateLimited => AuthConnectionState::RateLimited,
            Self::Missing | Self::Connecting | Self::BrowserOpened | Self::DeviceCodeWaiting => {
                AuthConnectionState::Missing
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

impl ReasoningEffort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
        }
    }
}

impl std::fmt::Display for ReasoningEffort {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiBlockedReason {
    CodexChatgptAuthRequired,
    CodexChatgptLoginPending,
    CodexChatgptExpired,
    CodexChatgptRefreshFailed,
    CodexChatgptUnsupported,
    CodexCliMissing,
    CodexVersionUnsupported,
    CodexAppServerUnavailable,
    CodexAppServerHandshakeFailed,
    CodexAuthModeUnsupported,
    CodexChatgptPlanLimited,
    CodexChatgptCreditsDepleted,
    CodexChatgptRateLimited,
    ModelListUnavailable,
    ModelUnavailable,
    ReasoningEffortUnavailable,
    PrivateDiffConsentRequired,
    GenerationAdapterUnavailable,
    CodexGenerationFailed,
    CodexGenerationInterrupted,
    CodexToolUseNotAllowed,
}

impl AiBlockedReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CodexChatgptAuthRequired => "codex_chatgpt_auth_required",
            Self::CodexChatgptLoginPending => "codex_chatgpt_login_pending",
            Self::CodexChatgptExpired => "codex_chatgpt_expired",
            Self::CodexChatgptRefreshFailed => "codex_chatgpt_refresh_failed",
            Self::CodexChatgptUnsupported => "codex_chatgpt_unsupported",
            Self::CodexCliMissing => "codex_cli_missing",
            Self::CodexVersionUnsupported => "codex_version_unsupported",
            Self::CodexAppServerUnavailable => "codex_app_server_unavailable",
            Self::CodexAppServerHandshakeFailed => "codex_app_server_handshake_failed",
            Self::CodexAuthModeUnsupported => "codex_auth_mode_unsupported",
            Self::CodexChatgptPlanLimited => "codex_chatgpt_plan_limited",
            Self::CodexChatgptCreditsDepleted => "codex_chatgpt_credits_depleted",
            Self::CodexChatgptRateLimited => "codex_chatgpt_rate_limited",
            Self::ModelListUnavailable => "model_list_unavailable",
            Self::ModelUnavailable => "model_unavailable",
            Self::ReasoningEffortUnavailable => "reasoning_effort_unavailable",
            Self::PrivateDiffConsentRequired => "private_diff_consent_required",
            Self::GenerationAdapterUnavailable => "generation_adapter_unavailable",
            Self::CodexGenerationFailed => "codex_generation_failed",
            Self::CodexGenerationInterrupted => "codex_generation_interrupted",
            Self::CodexToolUseNotAllowed => "codex_tool_use_not_allowed",
        }
    }
}

impl std::fmt::Display for AiBlockedReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAccountView {
    pub account_id_hash: String,
    pub display_label: Option<String>,
    pub workspace_name: Option<String>,
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionStatusView {
    pub provider: String,
    pub status: AiConnectionStatus,
    pub auth_mode: Option<String>,
    pub account: Option<AiAccountView>,
    pub plan_type: Option<String>,
    pub last_refreshed_at: Option<String>,
    pub blocked_reason: Option<AiBlockedReason>,
}

impl AiConnectionStatusView {
    pub fn missing() -> Self {
        Self {
            provider: "codex_chatgpt".to_string(),
            status: AiConnectionStatus::Missing,
            auth_mode: None,
            account: None,
            plan_type: None,
            last_refreshed_at: None,
            blocked_reason: Some(AiBlockedReason::CodexChatgptAuthRequired),
        }
    }

    pub fn unsupported() -> Self {
        Self {
            provider: "codex_chatgpt".to_string(),
            status: AiConnectionStatus::Unsupported,
            auth_mode: None,
            account: None,
            plan_type: None,
            last_refreshed_at: None,
            blocked_reason: Some(AiBlockedReason::CodexChatgptUnsupported),
        }
    }

    pub fn connected(account: AiAccountView) -> Self {
        Self {
            provider: "codex_chatgpt".to_string(),
            status: AiConnectionStatus::Connected,
            auth_mode: Some("chatgpt".to_string()),
            plan_type: account.plan_type.clone(),
            account: Some(account),
            last_refreshed_at: None,
            blocked_reason: None,
        }
    }

    pub fn from_legacy_auth_state(state: AuthConnectionState) -> Self {
        match state {
            AuthConnectionState::Connected => Self::connected(AiAccountView {
                account_id_hash: "legacy-chatgpt-account".to_string(),
                display_label: Some("ChatGPT".to_string()),
                workspace_name: None,
                plan_type: None,
            }),
            AuthConnectionState::Expired => Self {
                status: AiConnectionStatus::Expired,
                blocked_reason: Some(AiBlockedReason::CodexChatgptExpired),
                ..Self::missing()
            },
            AuthConnectionState::ReauthRequired => Self {
                status: AiConnectionStatus::ReauthRequired,
                blocked_reason: Some(AiBlockedReason::CodexChatgptRefreshFailed),
                ..Self::missing()
            },
            AuthConnectionState::Unsupported => Self::unsupported(),
            AuthConnectionState::ModelUnavailable => Self {
                status: AiConnectionStatus::Connected,
                auth_mode: Some("chatgpt".to_string()),
                blocked_reason: Some(AiBlockedReason::ModelUnavailable),
                ..Self::missing()
            },
            AuthConnectionState::RateLimited => Self {
                status: AiConnectionStatus::RateLimited,
                blocked_reason: Some(AiBlockedReason::CodexChatgptRateLimited),
                ..Self::missing()
            },
            _ => Self::missing(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelUpgradeView {
    pub required_plan: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelView {
    pub id: String,
    pub display_name: String,
    pub is_default: bool,
    pub hidden: bool,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub supported_reasoning_efforts: Vec<ReasoningEffort>,
    pub default_reasoning_effort: ReasoningEffort,
    pub input_modalities: Vec<String>,
    pub upgrade: Option<AiModelUpgradeView>,
}

impl AiModelView {
    pub fn available(
        id: impl Into<String>,
        display_name: impl Into<String>,
        supported_reasoning_efforts: Vec<ReasoningEffort>,
        default_reasoning_effort: ReasoningEffort,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            is_default: false,
            hidden: false,
            available: true,
            unavailable_reason: None,
            supported_reasoning_efforts,
            default_reasoning_effort,
            input_modalities: vec!["text".to_string()],
            upgrade: None,
        }
    }

    pub fn unavailable(
        id: impl Into<String>,
        display_name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            is_default: false,
            hidden: false,
            available: false,
            unavailable_reason: Some(reason.into()),
            supported_reasoning_efforts: vec![ReasoningEffort::Low],
            default_reasoning_effort: ReasoningEffort::Low,
            input_modalities: vec!["text".to_string()],
            upgrade: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiRateLimitStatus {
    Unknown,
    Ok,
    RateLimited,
    CreditsDepleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRateLimitSnapshot {
    pub status: AiRateLimitStatus,
    pub checked_at: Option<String>,
    pub resets_at: Option<String>,
    pub blocked_reason: Option<AiBlockedReason>,
}

impl AiRateLimitSnapshot {
    pub fn unknown() -> Self {
        Self {
            status: AiRateLimitStatus::Unknown,
            checked_at: None,
            resets_at: None,
            blocked_reason: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRunRecordView {
    pub run_id: String,
    pub pull_request_id: String,
    pub repo_full_name: String,
    pub pull_number: u64,
    pub head_sha: String,
    pub diff_hash: String,
    pub selected_files: Vec<String>,
    pub provider: String,
    pub auth_mode: String,
    pub plan_type: Option<String>,
    pub model_id: String,
    pub reasoning_effort: ReasoningEffort,
    pub review_language: Locale,
    pub prompt_version: String,
    pub private_diff_consent_snapshot: bool,
    pub rate_limit_snapshot: Option<AiRateLimitSnapshot>,
    pub status: String,
    pub blocked_reason: Option<AiBlockedReason>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppCapability {
    pub app_shell_available: bool,
    pub review_queue_available: bool,
    pub pr_context_available: bool,
    pub ai_review_available: bool,
    pub manual_draft_available: bool,
    pub submit_available: bool,
    pub settings_available: bool,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatusInput {
    pub github: AuthConnectionState,
    pub chatgpt: AuthConnectionState,
    pub strict_auth_mode: bool,
    pub language_preferences: LanguagePreferences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatusContext {
    pub github: AuthConnectionState,
    pub ai_connection: AiConnectionStatusView,
    pub strict_auth_mode: bool,
    pub language_preferences: LanguagePreferences,
    pub ai_models: Vec<AiModelView>,
    pub selected_model: String,
    pub selected_reasoning_effort: ReasoningEffort,
    pub rate_limit: AiRateLimitSnapshot,
    pub generation_adapter_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppStatusView {
    pub github: AuthConnectionState,
    pub chatgpt: AuthConnectionState,
    pub startup_gate_required: bool,
    pub review_queue_available: bool,
    pub ai_review_available: bool,
    pub strict_auth_mode: bool,
    pub capabilities: AppCapability,
    pub language_preferences: LanguagePreferences,
    pub selected_model: String,
    pub selected_reasoning_depth: String,
    pub selected_reasoning_effort: ReasoningEffort,
    pub ai_connection: AiConnectionStatusView,
    pub ai_models: Vec<AiModelView>,
    pub ai_rate_limit: AiRateLimitSnapshot,
    pub generation_available: bool,
    pub generation_blocked_reason: Option<String>,
    pub ai_blocked_reason: Option<String>,
    pub commands: Vec<IpcCommandSpec>,
}

pub fn frontend_safe_app_status(github_connected: bool, chatgpt_connected: bool) -> AppStatusView {
    frontend_safe_app_status_from_input(AppStatusInput {
        github: if github_connected {
            AuthConnectionState::Connected
        } else {
            AuthConnectionState::Missing
        },
        chatgpt: if chatgpt_connected {
            AuthConnectionState::Connected
        } else {
            AuthConnectionState::Missing
        },
        strict_auth_mode: false,
        language_preferences: LanguagePreferences::for_os_locale("en-US"),
    })
}

pub fn frontend_safe_app_status_from_input(input: AppStatusInput) -> AppStatusView {
    let github_ready = input.github == AuthConnectionState::Connected;
    let chatgpt_ready = input.chatgpt == AuthConnectionState::Connected;
    let strict_blocked = input.strict_auth_mode && !chatgpt_ready;
    let app_shell_available = github_ready && !strict_blocked;
    let ai_review_available = app_shell_available && chatgpt_ready;
    let mut blocked_reasons = Vec::new();

    if !github_ready {
        blocked_reasons.push(auth_blocked_reason("github", input.github).to_string());
    }
    if strict_blocked {
        blocked_reasons.push(auth_blocked_reason("chatgpt", input.chatgpt).to_string());
    }
    if app_shell_available && !chatgpt_ready {
        blocked_reasons.push(auth_blocked_reason("chatgpt", input.chatgpt).to_string());
    }

    let capabilities = AppCapability {
        app_shell_available,
        review_queue_available: app_shell_available,
        pr_context_available: app_shell_available,
        ai_review_available,
        manual_draft_available: app_shell_available,
        submit_available: app_shell_available,
        settings_available: true,
        blocked_reasons,
    };

    AppStatusView {
        github: input.github,
        chatgpt: input.chatgpt,
        startup_gate_required: !capabilities.app_shell_available,
        review_queue_available: capabilities.review_queue_available,
        ai_review_available: capabilities.ai_review_available,
        strict_auth_mode: input.strict_auth_mode,
        capabilities,
        language_preferences: input.language_preferences,
        selected_model: "gpt-5.5".to_string(),
        selected_reasoning_depth: "medium".to_string(),
        selected_reasoning_effort: ReasoningEffort::Medium,
        ai_connection: AiConnectionStatusView::from_legacy_auth_state(input.chatgpt),
        ai_models: Vec::new(),
        ai_rate_limit: AiRateLimitSnapshot::unknown(),
        generation_available: ai_review_available,
        generation_blocked_reason: (!ai_review_available).then(|| {
            if !github_ready {
                "github_auth_required".to_string()
            } else {
                "codex_chatgpt_auth_required".to_string()
            }
        }),
        ai_blocked_reason: (!ai_review_available).then(|| {
            if !github_ready {
                "GitHub OAuth required".to_string()
            } else {
                "codex_chatgpt_auth_required".to_string()
            }
        }),
        commands: allowed_ipc_commands().to_vec(),
    }
}

pub fn frontend_safe_app_status_from_context(input: AppStatusContext) -> AppStatusView {
    let github_ready = input.github == AuthConnectionState::Connected;
    let ai_connected = input.ai_connection.status == AiConnectionStatus::Connected;
    let strict_blocked = input.strict_auth_mode && !ai_connected;
    let app_shell_available = github_ready && !strict_blocked;
    let model_validation = validate_ai_run_configuration(
        &input.ai_models,
        &input.selected_model,
        input.selected_reasoning_effort,
    );
    let rate_blocked = match input.rate_limit.status {
        AiRateLimitStatus::RateLimited => Some(AiBlockedReason::CodexChatgptRateLimited),
        AiRateLimitStatus::CreditsDepleted => Some(AiBlockedReason::CodexChatgptCreditsDepleted),
        AiRateLimitStatus::Unknown | AiRateLimitStatus::Ok => None,
    };
    let generation_blocked = if input.generation_adapter_available {
        None
    } else {
        Some(AiBlockedReason::GenerationAdapterUnavailable)
    };
    let ai_blocked = if !github_ready {
        Some(AiBlockedReason::CodexChatgptAuthRequired)
    } else if !ai_connected {
        input
            .ai_connection
            .blocked_reason
            .or(Some(match input.ai_connection.status {
                AiConnectionStatus::Connecting
                | AiConnectionStatus::BrowserOpened
                | AiConnectionStatus::DeviceCodeWaiting => {
                    AiBlockedReason::CodexChatgptLoginPending
                }
                AiConnectionStatus::Expired => AiBlockedReason::CodexChatgptExpired,
                AiConnectionStatus::RefreshFailed | AiConnectionStatus::ReauthRequired => {
                    AiBlockedReason::CodexChatgptRefreshFailed
                }
                AiConnectionStatus::Unsupported => AiBlockedReason::CodexChatgptUnsupported,
                AiConnectionStatus::RateLimited => AiBlockedReason::CodexChatgptRateLimited,
                AiConnectionStatus::Missing | AiConnectionStatus::Connected => {
                    AiBlockedReason::CodexChatgptAuthRequired
                }
            }))
    } else if input.ai_models.is_empty() {
        Some(AiBlockedReason::ModelListUnavailable)
    } else if let Err(reason) = model_validation {
        Some(reason)
    } else if let Some(reason) = rate_blocked {
        Some(reason)
    } else {
        generation_blocked
    };
    let ai_review_available = app_shell_available && ai_blocked.is_none();
    let mut blocked_reasons = Vec::new();

    if !github_ready {
        blocked_reasons.push(auth_blocked_reason("github", input.github).to_string());
    }
    if strict_blocked {
        if let Some(reason) = ai_blocked {
            blocked_reasons.push(reason.as_str().to_string());
        }
    }
    if app_shell_available && !ai_review_available {
        if let Some(reason) = ai_blocked {
            blocked_reasons.push(reason.as_str().to_string());
        }
    }

    let capabilities = AppCapability {
        app_shell_available,
        review_queue_available: app_shell_available,
        pr_context_available: app_shell_available,
        ai_review_available,
        manual_draft_available: app_shell_available,
        submit_available: app_shell_available,
        settings_available: true,
        blocked_reasons,
    };

    let ai_blocked_reason = ai_blocked.map(|reason| reason.as_str().to_string());

    AppStatusView {
        github: input.github,
        chatgpt: input.ai_connection.status.legacy_auth_state(),
        startup_gate_required: !capabilities.app_shell_available,
        review_queue_available: capabilities.review_queue_available,
        ai_review_available: capabilities.ai_review_available,
        strict_auth_mode: input.strict_auth_mode,
        capabilities,
        language_preferences: input.language_preferences,
        selected_model: input.selected_model,
        selected_reasoning_depth: input.selected_reasoning_effort.as_str().to_string(),
        selected_reasoning_effort: input.selected_reasoning_effort,
        ai_connection: input.ai_connection,
        ai_models: input.ai_models,
        ai_rate_limit: input.rate_limit,
        generation_available: ai_review_available,
        generation_blocked_reason: ai_blocked_reason.clone(),
        ai_blocked_reason,
        commands: allowed_ipc_commands().to_vec(),
    }
}

fn auth_blocked_reason(provider: &str, state: AuthConnectionState) -> &'static str {
    match (provider, state) {
        ("github", AuthConnectionState::Missing) => "github_auth_required",
        ("github", AuthConnectionState::Expired) => "github_expired",
        ("github", AuthConnectionState::ReauthRequired) => "github_reauth_required",
        ("github", AuthConnectionState::ScopeMissing) => "github_scope_missing",
        ("github", AuthConnectionState::SsoRequired) => "github_sso_required",
        ("github", AuthConnectionState::RateLimited) => "github_rate_limited",
        ("chatgpt", AuthConnectionState::Missing) => "codex_chatgpt_auth_required",
        ("chatgpt", AuthConnectionState::Expired) => "codex_chatgpt_expired",
        ("chatgpt", AuthConnectionState::ReauthRequired) => "codex_chatgpt_refresh_failed",
        ("chatgpt", AuthConnectionState::Unsupported) => "codex_chatgpt_unsupported",
        ("chatgpt", AuthConnectionState::ModelUnavailable) => "model_unavailable",
        ("chatgpt", AuthConnectionState::RateLimited) => "codex_chatgpt_rate_limited",
        _ => "auth_required",
    }
}

pub fn validate_ai_run_configuration(
    models: &[AiModelView],
    selected_model: &str,
    reasoning_effort: ReasoningEffort,
) -> std::result::Result<(), AiBlockedReason> {
    let model = models
        .iter()
        .find(|model| model.id == selected_model && !model.hidden)
        .ok_or(AiBlockedReason::ModelUnavailable)?;

    if !model.available {
        return Err(AiBlockedReason::ModelUnavailable);
    }
    if !model
        .supported_reasoning_efforts
        .contains(&reasoning_effort)
    {
        return Err(AiBlockedReason::ReasoningEffortUnavailable);
    }
    Ok(())
}

pub fn resolve_review_locale(
    preferences: &LanguagePreferences,
    repository: Option<&str>,
    repo_config_locale: Option<Locale>,
) -> Locale {
    repository
        .and_then(|name| preferences.repo_review_locale_overrides.get(name))
        .copied()
        .or(repo_config_locale)
        .unwrap_or(preferences.review_locale)
}

pub fn validate_external_url(raw_url: &str) -> Result<url::Url> {
    let url = url::Url::parse(raw_url)?;
    if url.scheme() != "https" {
        return Err(ReviewDeskError::InvalidPath(
            "external URL must use https".to_string(),
        ));
    }
    let allowed = matches!(
        url.host_str(),
        Some("github.com")
            | Some("api.github.com")
            | Some("chatgpt.com")
            | Some("chat.openai.com")
            | Some("auth.openai.com")
    );
    if !allowed {
        return Err(ReviewDeskError::InvalidPath(
            "external URL host is not allowlisted".to_string(),
        ));
    }
    Ok(url)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRisk {
    ReadOnly,
    WritesLocal,
    WritesRemote,
    OpensExternal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct IpcCommandSpec {
    pub name: &'static str,
    pub risk: CommandRisk,
    pub credential_exposure: bool,
}

pub fn allowed_ipc_commands() -> &'static [IpcCommandSpec] {
    &[
        IpcCommandSpec {
            name: "get_app_status",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "start_github_oauth",
            risk: CommandRisk::OpensExternal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "poll_github_oauth",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "cancel_github_oauth",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "logout_github",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "refresh_github_auth_status",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "start_chatgpt_oauth",
            risk: CommandRisk::OpensExternal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "poll_chatgpt_oauth",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "list_repositories",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "load_review_queue",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "collect_pr_context",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "set_private_diff_consent",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "list_analysis_runs",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "start_analysis_run",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "read_analysis_run",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "cancel_analysis_run",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "archive_analysis_run",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "generate_review_draft",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "create_draft_from_run",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "save_review_draft",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "read_review_draft",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "list_review_drafts",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "mark_active_draft",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "save_draft",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "prepare_submit_review",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "validate_inline_comments",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "confirm_submit_review",
            risk: CommandRisk::WritesRemote,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "open_external_url",
            risk: CommandRisk::OpensExternal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "get_ai_connection_status",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "get_codex_bridge_status",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "start_codex_chatgpt_login",
            risk: CommandRisk::OpensExternal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "poll_codex_chatgpt_login",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "cancel_codex_chatgpt_login",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "read_codex_account",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "list_ai_models",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "select_ai_model",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "read_codex_rate_limits",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "logout_codex_chatgpt",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "refresh_ai_account_status",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "start_agent_run",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "read_agent_run",
            risk: CommandRisk::ReadOnly,
            credential_exposure: false,
        },
        IpcCommandSpec {
            name: "cancel_agent_run",
            risk: CommandRisk::WritesLocal,
            credential_exposure: false,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitPreflightInput {
    pub github_connected: bool,
    pub write_scope_valid: bool,
    pub sso_required: bool,
    pub pr_open: bool,
    pub pr_merged: bool,
    pub expected_head_sha: String,
    pub current_head_sha: String,
    pub draft_body: String,
    pub event: ReviewEvent,
    pub explicit_verdict_confirmed: bool,
    pub private_diff_consent_required: bool,
    pub private_diff_consent_accepted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmitPreflightStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitPreflightView {
    pub status: SubmitPreflightStatus,
    pub blocked_reasons: Vec<String>,
    pub confirmation_id: Option<String>,
}

pub fn prepare_submit_review(input: SubmitPreflightInput) -> SubmitPreflightView {
    let mut blocked_reasons = Vec::new();

    if !input.github_connected {
        blocked_reasons.push("github_auth_required".to_string());
    }
    if !input.write_scope_valid {
        blocked_reasons.push("github_scope_insufficient".to_string());
    }
    if input.sso_required {
        blocked_reasons.push("github_sso_required".to_string());
    }
    if !input.pr_open {
        blocked_reasons.push("pr_closed".to_string());
    }
    if input.pr_merged {
        blocked_reasons.push("pr_merged".to_string());
    }
    if input.expected_head_sha != input.current_head_sha {
        blocked_reasons.push("head_changed".to_string());
    }
    if input.draft_body.trim().is_empty() {
        blocked_reasons.push("body_empty".to_string());
    }
    if input.event != ReviewEvent::Comment && !input.explicit_verdict_confirmed {
        blocked_reasons.push("explicit_verdict_confirmation_required".to_string());
    }
    if input.private_diff_consent_required && !input.private_diff_consent_accepted {
        blocked_reasons.push("private_diff_consent_required".to_string());
    }

    if blocked_reasons.is_empty() {
        SubmitPreflightView {
            status: SubmitPreflightStatus::Ready,
            blocked_reasons,
            confirmation_id: Some(confirmation_id(&input)),
        }
    } else {
        SubmitPreflightView {
            status: SubmitPreflightStatus::Blocked,
            blocked_reasons,
            confirmation_id: None,
        }
    }
}

fn confirmation_id(input: &SubmitPreflightInput) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.expected_head_sha.as_bytes());
    hasher.update(input.current_head_sha.as_bytes());
    hasher.update(input.draft_body.as_bytes());
    hasher.update(format!("{:?}", input.event).as_bytes());
    format!("submit-{:x}", hasher.finalize())[..23].to_string()
}

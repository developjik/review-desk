use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReviewDeskError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("url error: {0}")]
    Url(#[from] url::ParseError),
    #[error("glob error: {0}")]
    Glob(#[from] globset::Error),
    #[error("keychain error: {0}")]
    Keychain(String),
    #[error("ai provider blocked: {0}")]
    AiBlocked(String),
    #[error("github api error: {kind:?}: {message}")]
    GitHubApi {
        kind: GitHubErrorKind,
        message: String,
    },
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

pub type Result<T> = std::result::Result<T, ReviewDeskError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AiProviderStatus {
    Ready,
    AuthRequired,
    BlockedUnsupportedAuth,
    ModelListUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewRunStatus {
    Discovered,
    ContextCollecting,
    ContextFailed,
    AiBlockedUnsupportedAuth,
    Analyzing,
    AnalysisFailed,
    DraftReady,
    UserEditing,
    ReadyToSubmit,
    Submitting,
    SubmitFailed,
    Submitted,
    Retrying,
    Cancelled,
    Stale,
    Dismissed,
}

impl ReviewRunStatus {
    pub fn can_submit(self) -> bool {
        matches!(self, Self::ReadyToSubmit | Self::UserEditing)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewEvent {
    Comment,
    Approve,
    RequestChanges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GitHubErrorKind {
    AuthRequired,
    ScopeMissing,
    SsoRequired,
    RateLimited,
    SecondaryRateLimited,
    SearchRateLimited,
    NotFound,
    Network,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repository {
    pub owner: String,
    pub repo: String,
    pub full_name: String,
    pub private: bool,
    pub last_refreshed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestQueueItem {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub url: String,
    pub draft: bool,
    pub review_requested: bool,
    pub assigned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_reason: Option<String>,
    pub ci_status: Option<String>,
    pub changed_files_count: Option<u64>,
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestSnapshot {
    pub snapshot_id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub base_branch: String,
    pub head_branch: String,
    pub head_sha: String,
    pub state: String,
    pub merged: bool,
    pub additions: u64,
    pub deletions: u64,
    pub changed_files_metadata_path: Option<String>,
    pub collected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedFile {
    pub path: String,
    pub patch: Option<String>,
}

impl ChangedFile {
    pub fn new(path: impl Into<String>, patch: Option<&str>) -> Self {
        Self {
            path: path.into(),
            patch: patch.map(ToOwned::to_owned),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewRun {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub snapshot_id: Option<String>,
    pub head_sha: String,
    pub diff_hash: Option<String>,
    pub status: ReviewRunStatus,
    pub risk_level: Option<String>,
    pub suggested_verdict: Option<ReviewEvent>,
    pub summary: Option<String>,
    pub prompt_version: String,
    pub provider: String,
    pub model: Option<String>,
    pub reasoning_depth: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub id: String,
    pub review_run_id: String,
    pub rubric_type: String,
    pub severity: String,
    pub confidence: f32,
    pub title: String,
    pub body: String,
    pub evidence: String,
    pub path: Option<String>,
    pub line: Option<u64>,
    pub side: Option<String>,
    pub selected_for_draft: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewDraft {
    pub id: String,
    pub review_run_id: String,
    pub head_sha: String,
    pub body: String,
    pub event: ReviewEvent,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub github_review_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmittedReview {
    pub id: String,
    pub github_review_id: u64,
    pub event: ReviewEvent,
    pub body: String,
    pub submitted_at: DateTime<Utc>,
    pub commit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiAccount {
    pub provider: String,
    pub account_id: Option<String>,
    pub plan_type: Option<String>,
    pub auth_mode: String,
    pub selected_model: Option<String>,
    pub selected_reasoning_depth: Option<String>,
    pub last_model_sync_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiModel {
    pub id: String,
    pub display_name: String,
    pub supported_reasoning_depths: Vec<ReasoningDepth>,
    pub default_reasoning_depth: Option<ReasoningDepth>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningDepth {
    Fast,
    Balanced,
    Deep,
}

impl ReasoningDepth {
    pub fn provider_value(self) -> &'static str {
        match self {
            Self::Fast => "low",
            Self::Balanced => "medium",
            Self::Deep => "high",
        }
    }
}

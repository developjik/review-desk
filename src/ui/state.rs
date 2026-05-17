use crate::domain::{
    AiProviderStatus, PullRequestQueueItem, ReasoningDepth, ReviewEvent, ReviewRunStatus,
};
use crate::github::{PrReference, parse_pr_reference};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppViewState {
    pub auth: AuthViewState,
    pub provider: ProviderViewState,
    pub queue: QueueViewState,
    pub run: RunViewState,
    pub draft: DraftViewState,
    pub task: Option<TaskViewState>,
    pub dialog: Option<DialogState>,
    pub bottom_status: BottomStatusState,
    pub viewport: ViewportMode,
}

impl AppViewState {
    pub fn startup_auth_required() -> Self {
        Self {
            auth: AuthViewState::GitHubAuthRequired,
            provider: ProviderViewState::blocked(),
            queue: QueueViewState::default(),
            run: RunViewState::default(),
            draft: DraftViewState::default(),
            task: Some(TaskViewState {
                id: "startup_auth_required".to_string(),
                label: "Checking saved sessions".to_string(),
                cancellable: false,
                retryable: false,
                last_error: None,
            }),
            dialog: None,
            bottom_status: BottomStatusState {
                message: "GitHub OAuth 연결이 필요합니다.".to_string(),
                severity: StatusSeverity::Warning,
            },
            viewport: ViewportMode::Wide,
        }
    }

    pub fn sample_github_connected_ai_blocked() -> Self {
        let mut state = Self::startup_auth_required();
        state.auth = AuthViewState::GitHubConnected {
            login: "developjik".to_string(),
        };
        state.provider = ProviderViewState::blocked();
        state.queue = QueueViewState::sample_loaded();
        state.run = RunViewState::sample_ai_blocked();
        state.draft = DraftViewState {
            body: String::new(),
            event: ReviewEvent::Comment,
            explicit_event_selected: false,
            dirty: false,
            stale: false,
            pr_closed_or_merged: false,
            submitting: false,
            write_scope_valid: true,
            report_path: ".reviewdesk/reviews/company-payment-web-582.md".to_string(),
            submitted_review_id: None,
            last_error: None,
            generated_by_ai: false,
        };
        state.task = None;
        state.bottom_status = BottomStatusState {
            message: "GitHub connected. AI provider blocked; manual draft is available."
                .to_string(),
            severity: StatusSeverity::Warning,
        };
        state
    }

    pub fn can_refresh_queue(&self) -> bool {
        self.auth.is_github_connected()
    }

    pub fn can_select_pr(&self) -> bool {
        self.auth.is_github_connected()
    }

    pub fn can_edit_manual_draft(&self) -> bool {
        self.queue.selected_pr.is_some()
            && self.run.status == ReviewRunStatus::AiBlockedUnsupportedAuth
    }

    pub fn can_analyze(&self) -> bool {
        self.analyze_disabled_reason().is_none()
    }

    pub fn analyze_disabled_reason(&self) -> Option<String> {
        if !self.auth.is_github_connected() {
            return Some("GitHub auth required".to_string());
        }
        if self.queue.selected_pr.is_none() {
            return Some("Select a pull request first".to_string());
        }
        if self.provider.status != AiProviderStatus::Ready {
            return Some("AI provider blocked".to_string());
        }
        if self.run.private_diff_consent_required && !self.run.private_diff_consent_accepted {
            return Some("Private diff consent required".to_string());
        }
        None
    }

    pub fn submit_disabled_reason(&self) -> Option<SubmitDisabledReason> {
        match &self.auth {
            AuthViewState::SsoRequired { .. } => return Some(SubmitDisabledReason::SsoRequired),
            AuthViewState::ScopeMissing { .. } => {
                return Some(SubmitDisabledReason::GitHubWriteScopeMissing);
            }
            _ => {}
        }
        if !self.auth.is_github_connected() {
            return Some(SubmitDisabledReason::GitHubAuthRequired);
        }
        if self.queue.selected_pr.is_none() {
            return Some(SubmitDisabledReason::NoPullRequestSelected);
        }
        if self.draft.body.trim().is_empty() {
            return Some(SubmitDisabledReason::DraftBodyEmpty);
        }
        if self.draft.stale {
            return Some(SubmitDisabledReason::DraftStale);
        }
        if self.draft.pr_closed_or_merged {
            return Some(SubmitDisabledReason::PullRequestClosedOrMerged);
        }
        if self.draft.submitting {
            return Some(SubmitDisabledReason::Submitting);
        }
        if !self.draft.write_scope_valid {
            return Some(SubmitDisabledReason::GitHubWriteScopeMissing);
        }
        if self.draft.generated_by_ai
            && self.run.private_diff_consent_required
            && !self.run.private_diff_consent_accepted
        {
            return Some(SubmitDisabledReason::PrivateDiffConsentRequired);
        }
        if self.draft.event != ReviewEvent::Comment && !self.draft.explicit_event_selected {
            return Some(SubmitDisabledReason::ExplicitVerdictConfirmationRequired);
        }
        None
    }

    pub fn validate_direct_pr_input(input: &str) -> DirectPrValidation {
        match parse_pr_reference(input) {
            Ok(reference) => DirectPrValidation::Valid(reference),
            Err(error) => DirectPrValidation::Invalid(error.to_string()),
        }
    }

    pub fn select_pr(&mut self, pr_ref: String) {
        self.queue.selected_pr = Some(pr_ref);
        self.run.status = ReviewRunStatus::ContextCollecting;
        self.bottom_status = BottomStatusState {
            message: "Collecting PR context".to_string(),
            severity: StatusSeverity::Info,
        };
    }

    pub fn select_review_event(&mut self, event: ReviewEvent) {
        self.draft.event = event;
        self.draft.explicit_event_selected = event != ReviewEvent::Comment;
        self.draft.dirty = true;
    }

    pub fn open_submit_dialog(&mut self) {
        if let Some(reason) = self.submit_disabled_reason() {
            self.dialog = Some(DialogState::Error {
                title: "Submit unavailable".to_string(),
                message: reason.label().to_string(),
            });
            return;
        }

        self.dialog = Some(DialogState::SubmitConfirm {
            repository: self.run.repository_label.clone(),
            pull_request: self.queue.selected_pr.clone().unwrap_or_default(),
            event: self.draft.event,
            head_sha: self.run.head_sha_short.clone(),
            stale_check: "Safe to submit".to_string(),
        });
    }

    pub fn cancel_dialog(&mut self) {
        self.dialog = None;
    }

    pub fn confirm_submit_dialog(&mut self) {
        if let Some(DialogState::SubmitConfirm {
            repository,
            pull_request,
            event,
            head_sha,
            ..
        }) = self.dialog.clone()
        {
            self.draft.submitting = true;
            self.run.status = ReviewRunStatus::Submitting;
            self.dialog = Some(DialogState::Submitting {
                repository,
                pull_request,
                event,
                head_sha,
            });
        }
    }

    pub fn apply_sample_submitted(&mut self) {
        self.draft.submitting = false;
        self.draft.submitted_review_id = Some(99);
        self.run.status = ReviewRunStatus::Submitted;
        self.dialog = None;
        self.bottom_status = BottomStatusState {
            message: "Submitted #99".to_string(),
            severity: StatusSeverity::Success,
        };
    }

    pub fn apply_sample_submit_failed(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.draft.submitting = false;
        self.draft.last_error = Some(message.clone());
        self.run.status = ReviewRunStatus::SubmitFailed;
        self.dialog = None;
        self.bottom_status = BottomStatusState {
            message,
            severity: StatusSeverity::Error,
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthViewState {
    StartupChecking,
    GitHubAuthRequired,
    GitHubConnected { login: String },
    ScopeMissing { message: String },
    SsoRequired { message: String },
}

impl AuthViewState {
    pub fn is_github_connected(&self) -> bool {
        matches!(self, Self::GitHubConnected { .. })
    }

    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::StartupChecking => "Checking",
            Self::GitHubAuthRequired => "Auth required",
            Self::GitHubConnected { .. } => "Connected",
            Self::ScopeMissing { .. } => "Scope missing",
            Self::SsoRequired { .. } => "SSO required",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderViewState {
    pub status: AiProviderStatus,
    pub account_label: Option<String>,
    pub selected_model: String,
    pub selected_depth: ReasoningDepth,
    pub available_models: Vec<String>,
    pub disabled_reason: Option<String>,
}

impl ProviderViewState {
    pub fn blocked() -> Self {
        Self {
            status: AiProviderStatus::BlockedUnsupportedAuth,
            account_label: Some("official provider unavailable".to_string()),
            selected_model: "gpt-5.4".to_string(),
            selected_depth: ReasoningDepth::Balanced,
            available_models: vec!["gpt-5.4".to_string(), "gpt-5.4-mini".to_string()],
            disabled_reason: Some("Official ChatGPT OAuth provider is unavailable.".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueFilter {
    All,
    ReviewRequested,
    Assigned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueViewState {
    pub repositories: Vec<String>,
    pub repositories_loaded: bool,
    pub selected_repository: Option<String>,
    pub filter: QueueFilter,
    pub search: String,
    pub items: Vec<PullRequestQueueItem>,
    pub selected_pr: Option<String>,
    pub direct_pr_input: String,
    pub direct_pr_error: Option<String>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for QueueViewState {
    fn default() -> Self {
        Self {
            repositories: Vec::new(),
            repositories_loaded: false,
            selected_repository: None,
            filter: QueueFilter::All,
            search: String::new(),
            items: Vec::new(),
            selected_pr: None,
            direct_pr_input: String::new(),
            direct_pr_error: None,
            loading: false,
            error: None,
        }
    }
}

impl QueueViewState {
    pub fn sample_loaded() -> Self {
        Self {
            repositories: vec![
                "company/payment-web".to_string(),
                "company/design-system".to_string(),
            ],
            repositories_loaded: true,
            selected_repository: Some("company/payment-web".to_string()),
            filter: QueueFilter::All,
            search: String::new(),
            items: vec![
                PullRequestQueueItem {
                    owner: "company".to_string(),
                    repo: "payment-web".to_string(),
                    number: 582,
                    title: "Payment refactor with retry-safe session handling".to_string(),
                    author: "alice".to_string(),
                    url: "https://github.com/company/payment-web/pull/582".to_string(),
                    draft: false,
                    review_requested: true,
                    assigned: false,
                    review_reason: Some("review_requested".to_string()),
                    ci_status: Some("pass".to_string()),
                    changed_files_count: Some(26),
                    updated_at: None,
                    stale_status: None,
                },
                PullRequestQueueItem {
                    owner: "company".to_string(),
                    repo: "payment-web".to_string(),
                    number: 590,
                    title: "Button color token cleanup".to_string(),
                    author: "bob".to_string(),
                    url: "https://github.com/company/payment-web/pull/590".to_string(),
                    draft: false,
                    review_requested: false,
                    assigned: true,
                    review_reason: Some("assigned".to_string()),
                    ci_status: Some("pending".to_string()),
                    changed_files_count: Some(2),
                    updated_at: None,
                    stale_status: None,
                },
            ],
            selected_pr: Some("company/payment-web#582".to_string()),
            direct_pr_input: String::new(),
            direct_pr_error: None,
            loading: false,
            error: None,
        }
    }

    pub fn filtered_items(&self) -> Vec<&PullRequestQueueItem> {
        let search = self.search.to_ascii_lowercase();
        self.items
            .iter()
            .filter(|item| match self.filter {
                QueueFilter::All => true,
                QueueFilter::ReviewRequested => item.review_requested,
                QueueFilter::Assigned => item.assigned,
            })
            .filter(|item| {
                search.is_empty()
                    || item.title.to_ascii_lowercase().contains(&search)
                    || item.repo.to_ascii_lowercase().contains(&search)
                    || item.number.to_string().contains(&search)
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunViewState {
    pub status: ReviewRunStatus,
    pub active_tab: WorkspaceTab,
    pub repository_label: String,
    pub pr_title: String,
    pub author: String,
    pub base_head: String,
    pub head_sha_short: String,
    pub risk: String,
    pub ci: String,
    pub changed_files: u64,
    pub additions: u64,
    pub deletions: u64,
    pub suggested_event: ReviewEvent,
    pub private_diff_consent_required: bool,
    pub private_diff_consent_accepted: bool,
    pub overview: Vec<String>,
    pub findings: Vec<FindingView>,
    pub files: Vec<FileView>,
    pub checklist: Vec<String>,
    pub error: Option<String>,
}

impl Default for RunViewState {
    fn default() -> Self {
        Self {
            status: ReviewRunStatus::Discovered,
            active_tab: WorkspaceTab::Overview,
            repository_label: "No PR selected".to_string(),
            pr_title: "Select a pull request".to_string(),
            author: "-".to_string(),
            base_head: "-".to_string(),
            head_sha_short: "-".to_string(),
            risk: "unknown".to_string(),
            ci: "unknown".to_string(),
            changed_files: 0,
            additions: 0,
            deletions: 0,
            suggested_event: ReviewEvent::Comment,
            private_diff_consent_required: false,
            private_diff_consent_accepted: false,
            overview: Vec::new(),
            findings: Vec::new(),
            files: Vec::new(),
            checklist: Vec::new(),
            error: None,
        }
    }
}

impl RunViewState {
    pub fn sample_ai_blocked() -> Self {
        Self {
            status: ReviewRunStatus::AiBlockedUnsupportedAuth,
            active_tab: WorkspaceTab::Overview,
            repository_label: "company/payment-web#582".to_string(),
            pr_title: "Payment refactor with retry-safe session handling".to_string(),
            author: "alice".to_string(),
            base_head: "main -> feature/payment".to_string(),
            head_sha_short: "head-sha".to_string(),
            risk: "high".to_string(),
            ci: "pass".to_string(),
            changed_files: 26,
            additions: 412,
            deletions: 128,
            suggested_event: ReviewEvent::Comment,
            private_diff_consent_required: true,
            private_diff_consent_accepted: false,
            overview: vec![
                "Auth/session flow changed across the API client and store.".to_string(),
                "Review refresh retry handling before approving.".to_string(),
                "Start with src/lib/apiClient.ts and src/stores/authStore.ts.".to_string(),
            ],
            findings: vec![
                FindingView {
                    severity: "concern".to_string(),
                    confidence: "medium".to_string(),
                    title: "Refresh retry may loop".to_string(),
                    evidence: "API client interceptor handles 401 paths.".to_string(),
                    action: "Confirm refresh endpoint is excluded from retry.".to_string(),
                    selected_for_draft: true,
                },
                FindingView {
                    severity: "question".to_string(),
                    confidence: "low".to_string(),
                    title: "User state cleanup".to_string(),
                    evidence: "Auth store clears token path in partial context.".to_string(),
                    action: "Ask whether user state is cleared on refresh failure.".to_string(),
                    selected_for_draft: false,
                },
            ],
            files: vec![
                FileView {
                    path: "src/lib/apiClient.ts".to_string(),
                    status: "modified".to_string(),
                    reason: "priority review file".to_string(),
                    partial: false,
                },
                FileView {
                    path: "pnpm-lock.yaml".to_string(),
                    status: "excluded".to_string(),
                    reason: "lockfile noise".to_string(),
                    partial: false,
                },
            ],
            checklist: vec![
                "Confirm CI pass is for the current head SHA.".to_string(),
                "Check refresh failure, expired token, and concurrent request cases.".to_string(),
                "Review UX fallback when auth expires during a user action.".to_string(),
            ],
            error: Some("AI_BLOCKED_UNSUPPORTED_AUTH".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceTab {
    Overview,
    Findings,
    Files,
    Checklist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingView {
    pub severity: String,
    pub confidence: String,
    pub title: String,
    pub evidence: String,
    pub action: String,
    pub selected_for_draft: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileView {
    pub path: String,
    pub status: String,
    pub reason: String,
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftViewState {
    pub body: String,
    pub event: ReviewEvent,
    pub explicit_event_selected: bool,
    pub dirty: bool,
    pub stale: bool,
    pub pr_closed_or_merged: bool,
    pub submitting: bool,
    pub write_scope_valid: bool,
    pub report_path: String,
    pub submitted_review_id: Option<u64>,
    pub last_error: Option<String>,
    pub generated_by_ai: bool,
}

impl Default for DraftViewState {
    fn default() -> Self {
        Self {
            body: String::new(),
            event: ReviewEvent::Comment,
            explicit_event_selected: false,
            dirty: false,
            stale: false,
            pr_closed_or_merged: false,
            submitting: false,
            write_scope_valid: true,
            report_path: ".reviewdesk/reviews/<owner>-<repo>-<number>.md".to_string(),
            submitted_review_id: None,
            last_error: None,
            generated_by_ai: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskViewState {
    pub id: String,
    pub label: String,
    pub cancellable: bool,
    pub retryable: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogState {
    SubmitConfirm {
        repository: String,
        pull_request: String,
        event: ReviewEvent,
        head_sha: String,
        stale_check: String,
    },
    Submitting {
        repository: String,
        pull_request: String,
        event: ReviewEvent,
        head_sha: String,
    },
    Error {
        title: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BottomStatusState {
    pub message: String,
    pub severity: StatusSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSeverity {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportMode {
    Wide,
    Narrow(WorkspaceSection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSection {
    Queue,
    Review,
    Draft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectPrValidation {
    Valid(PrReference),
    Invalid(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitDisabledReason {
    GitHubAuthRequired,
    NoPullRequestSelected,
    DraftBodyEmpty,
    DraftStale,
    PullRequestClosedOrMerged,
    Submitting,
    GitHubWriteScopeMissing,
    SsoRequired,
    PrivateDiffConsentRequired,
    ExplicitVerdictConfirmationRequired,
}

impl SubmitDisabledReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::GitHubAuthRequired => "GitHub auth required",
            Self::NoPullRequestSelected => "Select a pull request first",
            Self::DraftBodyEmpty => "Draft body is empty",
            Self::DraftStale => "Draft is stale; refresh or re-analyze",
            Self::PullRequestClosedOrMerged => "Pull request is closed or merged",
            Self::Submitting => "Review is submitting",
            Self::GitHubWriteScopeMissing => "GitHub review write scope missing",
            Self::SsoRequired => "GitHub organization SSO authorization required",
            Self::PrivateDiffConsentRequired => {
                "Private diff consent required before submitting AI-generated draft"
            }
            Self::ExplicitVerdictConfirmationRequired => {
                "APPROVE or REQUEST_CHANGES requires explicit selection"
            }
        }
    }
}

pub fn required_snapshot_names() -> &'static [&'static str] {
    &[
        "startup_auth_required",
        "github_connected_ai_blocked",
        "queue_empty",
        "queue_loaded_with_selection",
        "direct_pr_invalid",
        "context_collecting",
        "consent_required_private_repo",
        "draft_ready",
        "stale_submit_blocked",
        "submit_confirm_dialog",
        "submit_failed_draft_preserved",
        "submitted_review_id",
        "oauth_required_first_run",
        "queue_loaded_no_pr_selected",
        "pr_context_collecting",
        "ai_ready_consent_required",
        "ai_blocked_manual_draft",
        "draft_generated_needs_review",
        "submit_disabled_stale_head",
        "submit_confirm_approve_explicit",
        "submitted_review_linked",
        "modern_visual_baseline",
        "long_content_resilience",
    ]
}

pub fn viewport_mode_for_width(current: ViewportMode, width: f32) -> ViewportMode {
    if width < crate::ui::theme::NARROW_BREAKPOINT {
        match current {
            ViewportMode::Narrow(section) => ViewportMode::Narrow(section),
            ViewportMode::Wide => ViewportMode::Narrow(WorkspaceSection::Queue),
        }
    } else {
        ViewportMode::Wide
    }
}

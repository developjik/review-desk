export type AuthConnectionState =
  | "connected"
  | "missing"
  | "expired"
  | "reauth_required"
  | "scope_missing"
  | "sso_required"
  | "unsupported"
  | "model_unavailable"
  | "rate_limited";

export type Locale = "en" | "ko";

export type AiConnectionStatus =
  | "missing"
  | "connecting"
  | "browser_opened"
  | "device_code_waiting"
  | "connected"
  | "expired"
  | "reauth_required"
  | "refresh_failed"
  | "unsupported"
  | "rate_limited";

export type ReasoningEffort = "minimal" | "low" | "medium" | "high" | "xhigh";

export type AiBlockedReason =
  | "codex_chatgpt_auth_required"
  | "codex_chatgpt_login_pending"
  | "codex_chatgpt_expired"
  | "codex_chatgpt_refresh_failed"
  | "codex_chatgpt_unsupported"
  | "codex_cli_missing"
  | "codex_version_unsupported"
  | "codex_app_server_unavailable"
  | "codex_app_server_handshake_failed"
  | "codex_auth_mode_unsupported"
  | "codex_chatgpt_plan_limited"
  | "codex_chatgpt_credits_depleted"
  | "codex_chatgpt_rate_limited"
  | "model_list_unavailable"
  | "model_unavailable"
  | "reasoning_effort_unavailable"
  | "private_diff_consent_required"
  | "generation_adapter_unavailable"
  | "codex_generation_failed"
  | "codex_generation_interrupted"
  | "codex_tool_use_not_allowed";

export interface LanguagePreferences {
  ui_locale: Locale;
  review_locale: Locale;
  repo_review_locale_overrides: Record<string, Locale>;
}

export interface AppCapabilityView {
  app_shell_available: boolean;
  review_queue_available: boolean;
  pr_context_available: boolean;
  ai_review_available: boolean;
  manual_draft_available: boolean;
  submit_available: boolean;
  settings_available: boolean;
  blocked_reasons: string[];
}

export type CommandRisk = "read_only" | "writes_local" | "writes_remote" | "opens_external";

export interface IpcCommandSpec {
  name: string;
  risk: CommandRisk;
  credential_exposure: boolean;
}

export interface AiAccountView {
  accountIdHash: string;
  displayLabel?: string | null;
  workspaceName?: string | null;
  planType?: string | null;
}

export interface AiConnectionStatusView {
  provider: "codex_chatgpt";
  status: AiConnectionStatus;
  authMode: "chatgpt" | "apikey" | "chatgpt_auth_tokens" | "amazon_bedrock" | "missing" | "unknown" | null;
  account?: AiAccountView | null;
  planType?: string | null;
  lastRefreshedAt?: string | null;
  blockedReason?: AiBlockedReason | null;
}

export interface CodexBridgeStatusView {
  transport: "app_server_stdio" | "exec_json" | "unavailable";
  codexPath?: string | null;
  version?: string | null;
  appServerAvailable: boolean;
  execAvailable: boolean;
  blockedReason?: AiBlockedReason | null;
  diagnostics: string[];
}

export interface AiModelUpgradeView {
  requiredPlan?: string | null;
  message: string;
}

export interface AiModelView {
  id: string;
  displayName: string;
  isDefault: boolean;
  hidden: boolean;
  available: boolean;
  unavailableReason?: string | null;
  supportedReasoningEfforts: ReasoningEffort[];
  defaultReasoningEffort: ReasoningEffort;
  inputModalities: string[];
  upgrade?: AiModelUpgradeView | null;
}

export interface AiRateLimitSnapshot {
  status: "unknown" | "ok" | "rate_limited" | "credits_depleted";
  checkedAt?: string | null;
  resetsAt?: string | null;
  blockedReason?: AiBlockedReason | null;
}

export interface AgentRunRecordView {
  run_id: string;
  pull_request_id: string;
  repo_full_name: string;
  pull_number: number;
  head_sha: string;
  diff_hash: string;
  selected_files: string[];
  provider: "codex_chatgpt";
  auth_mode: "chatgpt";
  plan_type?: string | null;
  model_id: string;
  reasoning_effort: ReasoningEffort;
  review_language: Locale;
  prompt_version: string;
  private_diff_consent_snapshot: boolean;
  rate_limit_snapshot?: AiRateLimitSnapshot | null;
  status: "blocked" | "queued" | "running" | "draft_ready" | "failed" | "stale";
  blocked_reason?: AiBlockedReason | null;
  started_at: string;
  completed_at?: string | null;
}

export interface AppStatusView {
  github: AuthConnectionState;
  chatgpt: AuthConnectionState;
  startup_gate_required: boolean;
  review_queue_available: boolean;
  ai_review_available: boolean;
  strict_auth_mode?: boolean;
  capabilities?: AppCapabilityView;
  language_preferences?: LanguagePreferences;
  selected_model: string;
  selected_reasoning_depth: string;
  selected_reasoning_effort?: ReasoningEffort;
  ai_connection?: AiConnectionStatusView;
  ai_models?: AiModelView[];
  ai_rate_limit?: AiRateLimitSnapshot;
  generation_available?: boolean;
  generation_blocked_reason?: AiBlockedReason | string | null;
  ai_blocked_reason: string | null;
  commands: IpcCommandSpec[];
}

export interface Repository {
  owner: string;
  repo: string;
  full_name: string;
  private: boolean;
  last_refreshed_at: string | null;
}

export interface PullRequestQueueItem {
  owner: string;
  repo: string;
  number: number;
  title: string;
  author: string;
  url: string;
  draft: boolean;
  review_requested: boolean;
  assigned: boolean;
  ci_status: string | null;
  changed_files_count: number | null;
  updated_at: string | null;
  review_reason?: string | null;
  stale_status?: string | null;
}

export interface ChangedFile {
  path: string;
  patch: string | null;
}

export type StaleStatus =
  | "fresh"
  | "unknown"
  | "head_changed"
  | "diff_changed"
  | "context_changed"
  | "pr_closed"
  | "pr_merged";

export type CiRollupState =
  | "success"
  | "pending"
  | "failure"
  | "cancelled"
  | "skipped"
  | "neutral"
  | "unknown";

export interface PullRequestContextPr {
  owner: string;
  repo: string;
  number: number;
  title: string;
  body?: string | null;
  author: string;
  url: string;
  state: string;
  draft: boolean;
  merged: boolean;
  base_branch: string;
  head_branch: string;
  base_sha?: string | null;
  head_sha: string;
  additions: number;
  deletions: number;
  labels: string[];
  updated_at?: string | null;
}

export interface ChangedFileContext extends ChangedFile {
  status: string;
  previous_path?: string | null;
  additions: number;
  deletions: number;
  changes: number;
  patch_coverage: "available" | "missing" | "truncated" | string;
  patch_bytes: number;
  patch_hash?: string | null;
  is_generated: boolean;
  is_ignored_by_reviewdesk: boolean;
  ai_included: boolean;
  ui_only_reason?: string | null;
}

export interface ConversationSummary {
  issue_comment_count: number;
  review_comment_count: number;
  review_count: number;
  summaries: Array<{
    kind: string;
    author: string;
    created_at?: string | null;
    updated_at?: string | null;
    body_summary: string;
    unresolved: string;
    ai_included: boolean;
  }>;
  truncated: boolean;
}

export interface CiRollup {
  state: CiRollupState;
  source: string;
  failing_names: string[];
  pending_count: number;
  latest_completed_at?: string | null;
  warnings: string[];
}

export interface ReviewInputPreview {
  transmitted_scope: string;
  included_file_count: number;
  excluded_file_count: number;
  total_patch_bytes: number;
  redaction_passed: boolean;
  private_diff_consent_required: boolean;
}

export interface PullRequestContextView {
  pr: PullRequestContextPr;
  files: ChangedFileContext[];
  conversation: ConversationSummary;
  ci: CiRollup;
  freshness: StaleStatus;
  patch_coverage: "complete" | "partial" | "missing" | string;
  diff_hash: string;
  context_hash: string;
  collected_at: string;
  warnings: string[];
  ai_input: ReviewInputPreview;
}

export interface QueueSummary {
  ref: string;
  reason: string;
  ci: string;
  files: string;
}

export type InboxSectionId = "needs_review" | "blocked" | "drafts" | "recently_reviewed";
export type BadgeTone = "neutral" | "green" | "amber" | "red" | "blue";

export interface InboxSection {
  id: InboxSectionId;
  label: string;
  description: string;
  items: PullRequestQueueItem[];
}

export interface QueueGroupingState {
  draftRefs: Set<string>;
  submittedRefs: Set<string>;
}

export interface RiskBadge {
  id: string;
  label: string;
  tone: BadgeTone;
}

export type DiffLineType = "context" | "add" | "delete";

export interface DiffLine {
  type: DiffLineType;
  oldLine: number | null;
  newLine: number | null;
  content: string;
}

export interface DiffHunk {
  header: string;
  oldStart: number;
  newStart: number;
  lines: DiffLine[];
}

export interface SubmitSafetyInput {
  hasTarget: boolean;
  draftBody: string;
  draftDirtySincePreflight: boolean;
  preflight: { status: "ready" | "blocked"; blocked_reasons: string[]; confirmation_id: string | null } | null;
  submitting: boolean;
  submittedReviewId: number | null;
  submitMessage: string | null;
}

export type SubmitSafetyStatus =
  | "not_ready"
  | "stale"
  | "checking"
  | "blocked"
  | "ready"
  | "submitting"
  | "submitted"
  | "failed";

export interface SubmitSafetyState {
  status: SubmitSafetyStatus;
  label: string;
  blockers: string[];
  canPrepare: boolean;
  canConfirm: boolean;
}

export function startupGateLabel(status: AppStatusView): string {
  if (status.github !== "connected") {
    return "Connect GitHub in Browser to load your review inbox";
  }
  if (status.strict_auth_mode && status.chatgpt !== "connected") {
    return "Connect Codex to enter ReviewDesk";
  }
  if (status.chatgpt !== "connected") {
    return "Connect Codex to run AI review drafts";
  }
  return "ReviewDesk is ready";
}

export function commandDisabledReason(status: AppStatusView, command: string): string | null {
  if (command === "load_review_queue" || command === "list_repositories") {
    const available = status.capabilities?.review_queue_available ?? status.review_queue_available;
    if (available) return null;
    if (status.github !== "connected") return "GitHub OAuth required";
    if (status.strict_auth_mode && status.chatgpt !== "connected") {
      return status.ai_connection?.blockedReason ?? "codex_chatgpt_auth_required";
    }
    return "Review queue unavailable";
  }
  if (command === "generate_review_draft") {
    const available = status.capabilities?.ai_review_available ?? status.ai_review_available;
    return available
      ? null
      : (status.generation_blocked_reason ??
          status.ai_connection?.blockedReason ??
          status.ai_blocked_reason ??
          "AI unavailable");
  }
  if (command === "confirm_submit_review") {
    const available = status.capabilities?.submit_available ?? status.github === "connected";
    return available ? null : "GitHub OAuth required";
  }
  return null;
}

export function selectableAiModels(models: AiModelView[] | undefined): AiModelView[] {
  return (models ?? []).filter((model) => !model.hidden);
}

export function reasoningEffortsForModel(
  models: AiModelView[] | undefined,
  selectedModel: string,
): ReasoningEffort[] {
  return selectableAiModels(models).find((model) => model.id === selectedModel)?.supportedReasoningEfforts ?? [];
}

export function selectedAiModel(
  models: AiModelView[] | undefined,
  selectedModel: string,
): AiModelView | null {
  return selectableAiModels(models).find((model) => model.id === selectedModel) ?? null;
}

export function shouldShowStartupGate(status: AppStatusView): boolean {
  return status.startup_gate_required || !(status.capabilities?.app_shell_available ?? status.github === "connected");
}

export function defaultLanguagePreferences(osLocale: string): LanguagePreferences {
  const locale: Locale = osLocale.toLowerCase().startsWith("ko") ? "ko" : "en";
  return {
    ui_locale: locale,
    review_locale: locale,
    repo_review_locale_overrides: {},
  };
}

export function resolveReviewLocale(
  preferences: LanguagePreferences,
  repository: string | null,
  repoConfigLocale: Locale | null,
): Locale {
  if (repository && preferences.repo_review_locale_overrides[repository]) {
    return preferences.repo_review_locale_overrides[repository];
  }
  return repoConfigLocale ?? preferences.review_locale;
}

export function containsCredentialWords(payload: string): boolean {
  return /(\b(access[_-]?token|authorization|secret|bearer|refresh[_-]?token|client[_-]?secret|code[_-]?verifier|oauth[_-]?code)\b|["']code["']\s*:)/i.test(
    payload,
  );
}

export function summarizeQueueItem(item: PullRequestQueueItem): QueueSummary {
  const reasons =
    item.review_reason && item.review_reason !== "none"
      ? [item.review_reason.replace("_", " ")]
      : [
          item.review_requested ? "review requested" : null,
          item.assigned ? "assigned" : null,
          item.draft ? "draft" : null,
        ].filter(Boolean);

  return {
    ref: `${item.owner}/${item.repo}#${item.number}`,
    reason: reasons.length > 0 ? reasons.join(", ") : "open PR",
    ci: item.ci_status ?? "unknown",
    files: `${item.changed_files_count ?? 0} files`,
  };
}

export function pullRequestRef(item: Pick<PullRequestQueueItem, "owner" | "repo" | "number">): string {
  return `${item.owner}/${item.repo}#${item.number}`;
}

export function groupQueueBySection(
  queue: PullRequestQueueItem[],
  state: QueueGroupingState = { draftRefs: new Set(), submittedRefs: new Set() },
): InboxSection[] {
  return [
    {
      id: "needs_review",
      label: "Needs review",
      description: "Open pull requests requesting your attention.",
      items: queue.filter((item) => item.review_requested || item.assigned),
    },
    {
      id: "blocked",
      label: "Blocked",
      description: "Reviews with failing CI, stale context, or missing patches.",
      items: queue.filter((item) =>
        prRiskBadges(item).some((badge) => badge.tone === "red" || badge.id === "stale"),
      ),
    },
    {
      id: "drafts",
      label: "Drafts",
      description: "Pull requests with local or AI review draft work.",
      items: queue.filter((item) => state.draftRefs.has(pullRequestRef(item))),
    },
    {
      id: "recently_reviewed",
      label: "Recently reviewed",
      description: "Reviews submitted in this session.",
      items: queue.filter((item) => state.submittedRefs.has(pullRequestRef(item))),
    },
  ];
}

export function prRiskBadges(item: PullRequestQueueItem): RiskBadge[] {
  const badges: RiskBadge[] = [];
  if (item.ci_status === "fail") {
    badges.push({ id: "ci_failed", label: "CI failed", tone: "red" });
  }
  if (item.stale_status && item.stale_status !== "fresh") {
    badges.push({ id: "stale", label: staleStatusLabel(item.stale_status), tone: "amber" });
  }
  if ((item.changed_files_count ?? 0) >= 20) {
    badges.push({ id: "large_pr", label: `${item.changed_files_count} files`, tone: "amber" });
  }
  if (item.draft) {
    badges.push({ id: "draft_pr", label: "Draft PR", tone: "blue" });
  }
  return badges;
}

export function staleStatusLabel(status: string): string {
  const words = status.split("_");
  return words
    .map((part, index) => (index === 0 ? part.charAt(0).toUpperCase() + part.slice(1) : part))
    .join(" ");
}

export function reviewSearchPlaceholder(locale: Locale): string {
  return locale === "ko" ? "PR, repo, 제목, 파일 검색" : "Search PRs, repos, titles, or files";
}

export function parseUnifiedPatch(patch: string | null): DiffHunk[] {
  if (!patch) return [];
  const hunks: DiffHunk[] = [];
  let current: DiffHunk | null = null;
  let oldLine = 0;
  let newLine = 0;

  for (const rawLine of patch.split("\n")) {
    const header = rawLine.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
    if (header) {
      oldLine = Number(header[1]);
      newLine = Number(header[2]);
      current = { header: rawLine, oldStart: oldLine, newStart: newLine, lines: [] };
      hunks.push(current);
      continue;
    }
    if (!current) continue;
    if (rawLine.startsWith("+")) {
      current.lines.push({ type: "add", oldLine: null, newLine, content: rawLine.slice(1) });
      newLine += 1;
    } else if (rawLine.startsWith("-")) {
      current.lines.push({ type: "delete", oldLine, newLine: null, content: rawLine.slice(1) });
      oldLine += 1;
    } else {
      current.lines.push({
        type: "context",
        oldLine,
        newLine,
        content: rawLine.startsWith(" ") ? rawLine.slice(1) : rawLine,
      });
      oldLine += 1;
      newLine += 1;
    }
  }

  return hunks;
}

export function deriveSubmitSafetyState(input: SubmitSafetyInput): SubmitSafetyState {
  if (input.submittedReviewId) {
    return {
      status: "submitted",
      label: `Submitted review #${input.submittedReviewId}`,
      blockers: [],
      canPrepare: false,
      canConfirm: false,
    };
  }
  if (input.submitting) {
    return {
      status: "submitting",
      label: "Submitting review",
      blockers: [],
      canPrepare: false,
      canConfirm: false,
    };
  }
  if (!input.hasTarget || !input.draftBody.trim()) {
    return {
      status: "not_ready",
      label: "Select a PR and write a draft",
      blockers: ["body_empty"],
      canPrepare: input.hasTarget,
      canConfirm: false,
    };
  }
  if (input.draftDirtySincePreflight || !input.preflight) {
    return {
      status: "stale",
      label: "Prepare submit to refresh safety checks",
      blockers: [],
      canPrepare: true,
      canConfirm: false,
    };
  }
  if (input.preflight.status === "blocked") {
    return {
      status: "blocked",
      label: "Submit blocked",
      blockers: input.preflight.blocked_reasons,
      canPrepare: true,
      canConfirm: false,
    };
  }
  return {
    status: "ready",
    label: "Ready to submit",
    blockers: [],
    canPrepare: true,
    canConfirm: Boolean(input.preflight.confirmation_id),
  };
}

export function sampleStatus(): AppStatusView {
  return {
    github: "missing",
    chatgpt: "missing",
    startup_gate_required: true,
    review_queue_available: false,
    ai_review_available: false,
    strict_auth_mode: false,
    capabilities: {
      app_shell_available: false,
      review_queue_available: false,
      pr_context_available: false,
      ai_review_available: false,
      manual_draft_available: false,
      submit_available: false,
      settings_available: true,
      blocked_reasons: ["github_auth_required"],
    },
    language_preferences: defaultLanguagePreferences(
      typeof navigator === "undefined" ? "en-US" : navigator.language,
    ),
    selected_model: "gpt-5.5",
    selected_reasoning_depth: "medium",
    selected_reasoning_effort: "medium",
    ai_connection: {
      provider: "codex_chatgpt",
      status: "missing",
      authMode: null,
      account: null,
      planType: null,
      lastRefreshedAt: null,
      blockedReason: "codex_chatgpt_auth_required",
    },
    ai_models: [],
    ai_rate_limit: {
      status: "unknown",
      checkedAt: null,
      resetsAt: null,
      blockedReason: null,
    },
    generation_available: false,
    generation_blocked_reason: "codex_chatgpt_auth_required",
    ai_blocked_reason: "codex_chatgpt_auth_required",
    commands: [],
  };
}

export function sampleConnectedStatus(): AppStatusView {
  return {
    github: "connected",
    chatgpt: "missing",
    startup_gate_required: false,
    review_queue_available: true,
    ai_review_available: false,
    strict_auth_mode: false,
    capabilities: {
      app_shell_available: true,
      review_queue_available: true,
      pr_context_available: true,
      ai_review_available: false,
      manual_draft_available: true,
      submit_available: true,
      settings_available: true,
      blocked_reasons: ["codex_chatgpt_auth_required"],
    },
    language_preferences: defaultLanguagePreferences(
      typeof navigator === "undefined" ? "en-US" : navigator.language,
    ),
    selected_model: "gpt-5.5",
    selected_reasoning_depth: "medium",
    selected_reasoning_effort: "medium",
    ai_connection: {
      provider: "codex_chatgpt",
      status: "missing",
      authMode: null,
      account: null,
      planType: null,
      lastRefreshedAt: null,
      blockedReason: "codex_chatgpt_auth_required",
    },
    ai_models: [],
    ai_rate_limit: {
      status: "unknown",
      checkedAt: null,
      resetsAt: null,
      blockedReason: null,
    },
    generation_available: false,
    generation_blocked_reason: "codex_chatgpt_auth_required",
    ai_blocked_reason: "codex_chatgpt_auth_required",
    commands: [],
  };
}

export function sampleRepositories(): Repository[] {
  return [
    {
      owner: "company",
      repo: "payment-web",
      full_name: "company/payment-web",
      private: true,
      last_refreshed_at: null,
    },
    {
      owner: "company",
      repo: "design-system",
      full_name: "company/design-system",
      private: true,
      last_refreshed_at: null,
    },
  ];
}

export function sampleQueue(): PullRequestQueueItem[] {
  return [
    {
      owner: "company",
      repo: "payment-web",
      number: 582,
      title: "Payment refactor with retry-safe session handling",
      author: "alice",
      url: "https://github.com/company/payment-web/pull/582",
      draft: false,
      review_requested: true,
      assigned: false,
      ci_status: "pass",
      changed_files_count: 26,
      updated_at: null,
      review_reason: "review_requested",
      stale_status: "fresh",
    },
    {
      owner: "company",
      repo: "admin",
      number: 588,
      title: "User hook refactor",
      author: "morgan",
      url: "https://github.com/company/admin/pull/588",
      draft: false,
      review_requested: true,
      assigned: true,
      ci_status: "fail",
      changed_files_count: 8,
      updated_at: null,
      review_reason: "both",
      stale_status: "fresh",
    },
    {
      owner: "company",
      repo: "design-system",
      number: 590,
      title: "Button color token cleanup",
      author: "bob",
      url: "https://github.com/company/design-system/pull/590",
      draft: false,
      review_requested: false,
      assigned: true,
      ci_status: "pass",
      changed_files_count: 2,
      updated_at: null,
      review_reason: "assigned",
      stale_status: "fresh",
    },
  ];
}

export function sampleContext(item: PullRequestQueueItem): PullRequestContextView {
  return {
    pr: {
      owner: item.owner,
      repo: item.repo,
      number: item.number,
      title: item.title,
      body: "Sample ReviewDesk PR context used in demo mode.",
      author: item.author,
      url: item.url,
      base_branch: "main",
      head_branch: "feature/reviewdesk",
      base_sha: null,
      head_sha: "abc123def456",
      state: "open",
      draft: item.draft,
      merged: false,
      additions: 412,
      deletions: 128,
      labels: ["reviewdesk"],
      updated_at: item.updated_at,
    },
    files: [
      {
        path: "src/lib/apiClient.ts",
        patch:
          "@@ -70,7 +70,12 @@ export async function request(input) {\n-  return fetch(input)\n+  if (needsRefresh()) {\n+    await refreshSession()\n+  }\n+  return fetch(input)\n }",
        status: "modified",
        previous_path: null,
        additions: 5,
        deletions: 1,
        changes: 6,
        patch_coverage: "available",
        patch_bytes: 142,
        patch_hash: "sample-api-client",
        is_generated: false,
        is_ignored_by_reviewdesk: false,
        ai_included: true,
        ui_only_reason: null,
      },
      {
        path: "src/stores/authStore.ts",
        patch:
          "@@ -41,6 +41,8 @@ export function clearSession() {\n   token = null\n+  refreshInFlight = null\n+  lastRefreshError = null\n }",
        status: "modified",
        previous_path: null,
        additions: 2,
        deletions: 0,
        changes: 2,
        patch_coverage: "available",
        patch_bytes: 118,
        patch_hash: "sample-auth-store",
        is_generated: false,
        is_ignored_by_reviewdesk: false,
        ai_included: true,
        ui_only_reason: null,
      },
      {
        path: "pnpm-lock.yaml",
        patch: null,
        status: "modified",
        previous_path: null,
        additions: 405,
        deletions: 127,
        changes: 532,
        patch_coverage: "missing",
        patch_bytes: 0,
        patch_hash: null,
        is_generated: false,
        is_ignored_by_reviewdesk: true,
        ai_included: false,
        ui_only_reason: "patch_unavailable",
      },
    ],
    conversation: {
      issue_comment_count: 3,
      review_count: 1,
      review_comment_count: 2,
      summaries: [],
      truncated: false,
    },
    ci: {
      state: item.ci_status === "pass" ? "success" : item.ci_status === "fail" ? "failure" : "unknown",
      source: "demo",
      failing_names: item.ci_status === "fail" ? ["unit-tests"] : [],
      pending_count: 0,
      latest_completed_at: null,
      warnings: [],
    },
    freshness: "fresh",
    patch_coverage: "partial",
    diff_hash: "sample-diff-hash",
    context_hash: "sample-context-hash",
    collected_at: new Date().toISOString(),
    warnings: [],
    ai_input: {
      transmitted_scope: "selected_patches",
      included_file_count: 2,
      excluded_file_count: 1,
      total_patch_bytes: 260,
      redaction_passed: true,
      private_diff_consent_required: false,
    },
  };
}

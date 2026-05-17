import { invoke } from "@tauri-apps/api/core";
import { derivePublishBlockers } from "./publish-safety";
import {
  type AgentRunRecordView,
  type AiConnectionStatusView,
  type AiModelView,
  type AiRateLimitSnapshot,
  type AppStatusView,
  type ChangedFile,
  type CodexBridgeStatusView,
  type Locale,
  type ReasoningEffort,
  type PullRequestContextView,
  type PullRequestQueueItem,
  type Repository,
  sampleContext,
  sampleConnectedStatus,
  sampleQueue,
  sampleRepositories,
  sampleStatus,
} from "./view-models";
import type {
  AnalysisRunMode,
  AnalysisRunView,
  InlineCommentDraftView,
  InlineMappingStatus,
  ReviewDraftView,
  ReviewEvent,
  ReviewPublishPayloadView,
} from "./workspace-view-models";

export type {
  AnalysisRunMode,
  AnalysisRunStatus,
  AnalysisRunView,
  InlineMappingStatus,
  ReviewDraftView,
  ReviewEvent,
  ReviewPublishPayloadView,
} from "./workspace-view-models";

export type InlineCommentDraft = InlineCommentDraftView;
export type ReviewPublishPayload = ReviewPublishPayloadView;

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

function isDemoMode(): boolean {
  const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };
  return meta.env?.VITE_REVIEWDESK_DEMO_MODE === "1" || isReviewDeskDemoSearch(window.location.search);
}

export function isReviewDeskDemoSearch(search: string): boolean {
  return new URLSearchParams(search).get("reviewdesk_demo") === "1";
}

export function shouldUseDemoFallback(command: string, demoMode: boolean, tauriRuntime: boolean): boolean {
  if (command === "get_app_status" && !tauriRuntime) return true;
  return demoMode && !["confirm_submit_review", "open_external_url"].includes(command);
}

async function safeInvoke<T>(command: string, args: Record<string, unknown>, fallback: T): Promise<T> {
  const demoMode = isDemoMode();
  const tauriRuntime = isTauriRuntime();
  if (shouldUseDemoFallback(command, demoMode, tauriRuntime)) {
    return fallback;
  }
  if (!tauriRuntime) {
    throw new Error("Tauri runtime required. Enable VITE_REVIEWDESK_DEMO_MODE=1 for sample data.");
  }
  return invoke<T>(command, args);
}

async function safeInvokeNoFallback<T>(command: string, args: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error("Tauri runtime required.");
  }
  return invoke<T>(command, args);
}

export function getAppStatus(): Promise<AppStatusView> {
  return safeInvoke("get_app_status", {}, isDemoMode() ? sampleConnectedStatus() : sampleStatus());
}

export function listRepositories(): Promise<Repository[]> {
  return safeInvoke("list_repositories", {}, sampleRepositories());
}

export function loadReviewQueue(repository: string | null): Promise<PullRequestQueueItem[]> {
  return safeInvoke("load_review_queue", { request: { repository } }, sampleQueue());
}

export function collectPrContext(item: PullRequestQueueItem): Promise<PullRequestContextView> {
  return safeInvoke(
    "collect_pr_context",
    {
      request: {
        owner: item.owner,
        repo: item.repo,
        number: item.number,
      },
    },
    sampleContext(item),
  );
}

export interface SubmitPreflightInput {
  payload: ReviewPublishPayload;
  github_connected: boolean;
  write_scope_valid: boolean;
  sso_required: boolean;
  pr_open: boolean;
  pr_merged: boolean;
  current_head_sha: string;
  current_diff_hash: string;
}

export interface SubmitPreflightView {
  status: "ready" | "blocked";
  blocked_reasons: string[];
  confirmation_id: string | null;
  payload?: ReviewPublishPayload;
}

export interface ConfirmSubmitReviewInput {
  payload: ReviewPublishPayload;
  confirmation_id: string;
}

export interface SubmittedReviewView {
  id: number;
  url: string;
  payload?: ReviewPublishPayload;
}

export function buildReviewPublishPayload(input: ReviewPublishPayload): ReviewPublishPayload {
  return {
    ...input,
    inline_comments: input.inline_comments,
  };
}

export function buildPrepareSubmitReviewRequest(input: SubmitPreflightInput): SubmitPreflightInput {
  return {
    ...input,
    payload: buildReviewPublishPayload(input.payload),
  };
}

export function buildConfirmSubmitReviewRequest(input: ConfirmSubmitReviewInput): ConfirmSubmitReviewInput {
  return {
    ...input,
    payload: buildReviewPublishPayload(input.payload),
  };
}

export function prepareSubmitReview(input: SubmitPreflightInput): Promise<SubmitPreflightView> {
  const blockedReasons = derivePublishBlockers(input.payload, {
    github_connected: input.github_connected,
    write_scope_valid: input.write_scope_valid,
    sso_required: input.sso_required,
    pr_open: input.pr_open,
    pr_merged: input.pr_merged,
    current_head_sha: input.current_head_sha,
    current_diff_hash: input.current_diff_hash,
  });

  return safeInvoke("prepare_submit_review", { request: buildPrepareSubmitReviewRequest(input) }, {
    status: blockedReasons.length === 0 ? "ready" : "blocked",
    blocked_reasons: blockedReasons,
    confirmation_id: blockedReasons.length === 0 ? "sample-confirmation" : null,
    payload: input.payload,
  });
}

export function confirmSubmitReview(input: ConfirmSubmitReviewInput): Promise<SubmittedReviewView> {
  return safeInvokeNoFallback("confirm_submit_review", { request: buildConfirmSubmitReviewRequest(input) });
}

export interface OAuthStartView {
  flow_id: string;
  status: string;
  mode: "browser" | "device" | string;
  auth_url: string | null;
  redirect_uri: string | null;
  user_code: string | null;
  verification_uri: string | null;
  expires_in: number | null;
  interval: number | null;
  disabled_reason: string | null;
}

export interface OAuthPollView {
  flow_id: string | null;
  status: string;
  mode: "browser" | "device" | string | null;
  account_login: string | null;
  user_code: string | null;
  verification_uri: string | null;
  disabled_reason: string | null;
}

export interface CodexChatGptLoginStartView {
  login_id: string;
  status: string;
  mode: "browser" | "device";
  auth_url: string | null;
  verification_url: string | null;
  user_code: string | null;
  disabled_reason: string | null;
}

export interface AiModelListView {
  status: "ready" | "blocked";
  models: AiModelView[];
  disabled_reason: string | null;
}

export interface GenerateReviewDraftInput {
  owner: string;
  repo: string;
  number: number;
  files: ChangedFile[];
  model: string;
  reasoning_depth: string;
  reasoning_effort: ReasoningEffort;
  review_language: Locale;
  head_sha: string | null;
  private_diff_consent_required: boolean;
  private_diff_consent_accepted: boolean;
}

export interface GeneratedDraftView {
  status: string;
  body: string;
  report_path: string | null;
  disabled_reason: string | null;
  blocked_reason: string | null;
  run: AgentRunRecordView | null;
}

export interface AgentRunStateView {
  run: AgentRunRecordView;
  draft_body?: string | null;
  report_path?: string | null;
  disabled_reason?: string | null;
}

export interface PullRequestWorkspaceInput {
  owner: string;
  repo: string;
  number: number;
}

export interface AnalysisRunReferenceInput extends PullRequestWorkspaceInput {
  run_id: string;
}

export interface StartAnalysisRunInput extends PullRequestWorkspaceInput {
  files: ChangedFile[];
  mode: AnalysisRunMode;
  model: string;
  reasoning_effort: ReasoningEffort;
  review_language: Locale;
  head_sha: string | null;
  diff_hash?: string | null;
  context_hash?: string | null;
  custom_prompt?: string | null;
  selected_files?: string[];
  excluded_files?: string[];
  private_diff_consent_required: boolean;
  private_diff_consent_accepted: boolean;
}

export interface CreateDraftFromRunInput extends PullRequestWorkspaceInput {
  source_run_ids: string[];
  base_head_sha: string;
  base_diff_hash: string;
  body: string;
  event: ReviewEvent;
}

export interface SaveReviewDraftInput {
  draft: ReviewDraftView;
  mark_active: boolean;
}

export interface ReadReviewDraftInput extends PullRequestWorkspaceInput {
  draft_id: string;
}

export interface MarkActiveDraftInput extends PullRequestWorkspaceInput {
  draft_id: string;
}

export interface ValidateInlineCommentsInput {
  comments: InlineCommentDraft[];
  files: ChangedFile[];
  diff_hash: string;
}

export function startGithubOAuth(mode: "browser" | "device" = "browser"): Promise<OAuthStartView> {
  return safeInvokeNoFallback("start_github_oauth", { request: { client_id: null, mode } });
}

export function pollGithubOAuth(flow_id: string): Promise<OAuthPollView> {
  return safeInvokeNoFallback("poll_github_oauth", { request: { flow_id } });
}

export function refreshGithubAuthStatus(): Promise<OAuthPollView> {
  return safeInvokeNoFallback("refresh_github_auth_status", {});
}

export function startChatGptOAuth(): Promise<OAuthStartView> {
  return safeInvokeNoFallback("start_chatgpt_oauth", { request: { client_id: null } });
}

export function getAiConnectionStatus(): Promise<AiConnectionStatusView> {
  return safeInvokeNoFallback("get_ai_connection_status", {});
}

export function getCodexBridgeStatus(): Promise<CodexBridgeStatusView> {
  return safeInvokeNoFallback("get_codex_bridge_status", {});
}

export function startCodexChatGptLogin(
  mode: "browser" | "device",
): Promise<CodexChatGptLoginStartView> {
  return safeInvokeNoFallback("start_codex_chatgpt_login", { request: { mode } });
}

export function pollCodexChatGptLogin(login_id: string): Promise<OAuthPollView> {
  return safeInvokeNoFallback("poll_codex_chatgpt_login", { request: { login_id } });
}

export function listAiModels(): Promise<AiModelListView> {
  return safeInvoke("list_ai_models", {}, {
    status: "blocked",
    models: [],
    disabled_reason: "model_list_unavailable",
  });
}

export function selectAiModel(model: string, reasoning_effort: ReasoningEffort): Promise<AppStatusView> {
  return safeInvokeNoFallback("select_ai_model", { request: { model, reasoning_effort } });
}

export function readCodexRateLimits(): Promise<AiRateLimitSnapshot> {
  return safeInvokeNoFallback("read_codex_rate_limits", {});
}

export function refreshAiAccountStatus(): Promise<AiConnectionStatusView> {
  return safeInvokeNoFallback("refresh_ai_account_status", {});
}

export function generateReviewDraft(input: GenerateReviewDraftInput): Promise<GeneratedDraftView> {
  return safeInvokeNoFallback("generate_review_draft", { request: input });
}

export function startAgentRun(input: GenerateReviewDraftInput): Promise<GeneratedDraftView> {
  return safeInvokeNoFallback("start_agent_run", { request: input });
}

export function readAgentRun(run_id: string): Promise<AgentRunStateView> {
  return safeInvokeNoFallback("read_agent_run", { request: { run_id } });
}

export function cancelAgentRun(run_id: string): Promise<AgentRunStateView> {
  return safeInvokeNoFallback("cancel_agent_run", { request: { run_id } });
}

export function listAnalysisRuns(input: PullRequestWorkspaceInput): Promise<AnalysisRunView[]> {
  return safeInvokeNoFallback("list_analysis_runs", { request: input });
}

export function startAnalysisRun(input: StartAnalysisRunInput): Promise<AnalysisRunView> {
  return safeInvokeNoFallback("start_analysis_run", { request: input });
}

export function readAnalysisRun(input: AnalysisRunReferenceInput): Promise<AnalysisRunView> {
  return safeInvokeNoFallback("read_analysis_run", { request: input });
}

export function cancelAnalysisRun(input: AnalysisRunReferenceInput): Promise<AnalysisRunView> {
  return safeInvokeNoFallback("cancel_analysis_run", { request: input });
}

export function archiveAnalysisRun(input: AnalysisRunReferenceInput): Promise<AnalysisRunView> {
  return safeInvokeNoFallback("archive_analysis_run", { request: input });
}

export function createDraftFromRun(input: CreateDraftFromRunInput): Promise<ReviewDraftView> {
  return safeInvokeNoFallback("create_draft_from_run", { request: input });
}

export function saveReviewDraft(input: SaveReviewDraftInput): Promise<ReviewDraftView> {
  return safeInvokeNoFallback("save_review_draft", { request: input });
}

export function readReviewDraft(input: ReadReviewDraftInput): Promise<ReviewDraftView> {
  return safeInvokeNoFallback("read_review_draft", { request: input });
}

export function listReviewDrafts(input: PullRequestWorkspaceInput): Promise<ReviewDraftView[]> {
  return safeInvokeNoFallback("list_review_drafts", { request: input });
}

export function markActiveDraft(input: MarkActiveDraftInput): Promise<string | null> {
  return safeInvokeNoFallback("mark_active_draft", { request: input });
}

export function validateInlineComments(input: ValidateInlineCommentsInput): Promise<InlineCommentDraft[]> {
  return safeInvokeNoFallback("validate_inline_comments", { request: input });
}

export function openExternalUrl(url: string): Promise<void> {
  return safeInvokeNoFallback("open_external_url", { request: { url } });
}

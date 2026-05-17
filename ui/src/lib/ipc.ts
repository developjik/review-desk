import { invoke } from "@tauri-apps/api/core";
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
  github_connected: boolean;
  write_scope_valid: boolean;
  sso_required: boolean;
  pr_open: boolean;
  pr_merged: boolean;
  expected_head_sha: string;
  current_head_sha: string;
  draft_body: string;
  event: "COMMENT" | "APPROVE" | "REQUEST_CHANGES";
  explicit_verdict_confirmed: boolean;
  private_diff_consent_required: boolean;
  private_diff_consent_accepted: boolean;
}

export interface SubmitPreflightView {
  status: "ready" | "blocked";
  blocked_reasons: string[];
  confirmation_id: string | null;
}

export interface ConfirmSubmitReviewInput {
  owner: string;
  repo: string;
  number: number;
  expected_head_sha: string;
  body: string;
  event: "COMMENT" | "APPROVE" | "REQUEST_CHANGES";
  explicit_verdict_confirmed: boolean;
  confirmation_id: string;
}

export interface SubmittedReviewView {
  id: number;
  url: string;
}

export function prepareSubmitReview(input: SubmitPreflightInput): Promise<SubmitPreflightView> {
  const blockedReasons: string[] = [];
  if (!input.github_connected) blockedReasons.push("github_auth_required");
  if (!input.write_scope_valid) blockedReasons.push("github_scope_insufficient");
  if (input.sso_required) blockedReasons.push("github_sso_required");
  if (!input.pr_open) blockedReasons.push("pr_closed");
  if (input.pr_merged) blockedReasons.push("pr_merged");
  if (input.expected_head_sha !== input.current_head_sha) blockedReasons.push("head_changed");
  if (!input.draft_body.trim()) blockedReasons.push("body_empty");
  if (input.event !== "COMMENT" && !input.explicit_verdict_confirmed) {
    blockedReasons.push("explicit_verdict_confirmation_required");
  }
  if (input.private_diff_consent_required && !input.private_diff_consent_accepted) {
    blockedReasons.push("private_diff_consent_required");
  }

  return safeInvoke("prepare_submit_review", { request: input }, {
    status: blockedReasons.length === 0 ? "ready" : "blocked",
    blocked_reasons: blockedReasons,
    confirmation_id: blockedReasons.length === 0 ? "sample-confirmation" : null,
  });
}

export function confirmSubmitReview(input: ConfirmSubmitReviewInput): Promise<SubmittedReviewView> {
  return safeInvokeNoFallback("confirm_submit_review", { request: input });
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

export function openExternalUrl(url: string): Promise<void> {
  return safeInvokeNoFallback("open_external_url", { request: { url } });
}

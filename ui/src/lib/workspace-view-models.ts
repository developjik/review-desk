import type {
  AiBlockedReason,
  Locale,
  PullRequestQueueItem,
  ReasoningEffort,
} from "./view-models";

export type ReviewEvent = "COMMENT" | "APPROVE" | "REQUEST_CHANGES";
export type AnalysisRunMode = "fast" | "deep" | "security" | "tests" | "custom";
export type AnalysisRunStatus =
  | "queued"
  | "running"
  | "draft_ready"
  | "failed"
  | "cancelled"
  | "stale"
  | "archived";
export type InlineMappingStatus = "valid" | "missing_path" | "invalid_side" | "invalid_line" | "stale_diff";

export interface ReviewFindingView {
  id?: string;
  path?: string | null;
  line?: number | null;
  severity?: string | null;
  confidence?: number | null;
  title?: string | null;
  body?: string | null;
  [key: string]: unknown;
}

export interface AnalysisRunView {
  run_id: string;
  owner: string;
  repo: string;
  number: number;
  head_sha: string;
  diff_hash: string;
  context_hash: string;
  mode: AnalysisRunMode;
  model_id: string;
  reasoning_effort: ReasoningEffort;
  review_language: Locale;
  custom_prompt: string | null;
  prompt_version: string;
  selected_files: string[];
  excluded_files: string[];
  private_diff_consent_snapshot: boolean;
  status: AnalysisRunStatus;
  blocked_reason: AiBlockedReason | null;
  draft_seed_body: string | null;
  findings: ReviewFindingView[];
  created_at: string;
  completed_at: string | null;
}

export interface InlineCommentDraftView {
  id: string;
  path: string;
  side: string;
  line: number;
  start_line: number | null;
  start_side: string | null;
  body: string;
  severity: string | null;
  confidence: number | null;
  source_run_id: string | null;
  source_finding_id: string | null;
  selected_for_publish: boolean;
  dismissed: boolean;
  user_edited: boolean;
  mapping_status: InlineMappingStatus;
}

export interface ReviewDraftView {
  draft_id: string;
  owner: string;
  repo: string;
  number: number;
  source_run_ids: string[];
  base_head_sha: string;
  base_diff_hash: string;
  verdict: ReviewEvent;
  body: string;
  inline_comments: InlineCommentDraftView[];
  user_edited: boolean;
  stale: boolean;
  created_at: string;
  updated_at: string;
}

export interface ReviewPublishPayloadView {
  owner: string;
  repo: string;
  number: number;
  expected_head_sha: string;
  expected_diff_hash: string;
  event: ReviewEvent;
  body: string;
  inline_comments: InlineCommentDraftView[];
  explicit_verdict_confirmed: boolean;
  private_diff_consent_required: boolean;
  private_diff_consent_accepted: boolean;
}

export interface PublishAttemptView {
  attempt_id: string;
  draft_id: string | null;
  confirmation_id: string;
  payload: ReviewPublishPayloadView;
  github_account: string | null;
  status: string;
  github_review_id: number | null;
  error_code: string | null;
  error_message: string | null;
  created_at: string;
  completed_at: string | null;
}

export type RunCentricQueueSectionId =
  | "needs_review"
  | "draft_ready"
  | "running"
  | "blocked_stale"
  | "published";

export interface RunCentricQueueItem extends PullRequestQueueItem {
  latest_run_id?: string | null;
  latest_run_status?: AnalysisRunStatus | null;
  active_draft_id?: string | null;
  active_draft_stale?: boolean | null;
  published_review_id?: number | null;
}

export interface RunCentricQueueSection {
  id: RunCentricQueueSectionId;
  label: string;
  description: string;
  items: RunCentricQueueItem[];
}

const RUN_CENTRIC_SECTIONS: Array<Omit<RunCentricQueueSection, "items">> = [
  {
    id: "needs_review",
    label: "Needs review",
    description: "Pull requests waiting for review work.",
  },
  {
    id: "draft_ready",
    label: "Draft ready",
    description: "Pull requests with review drafts ready to edit or publish.",
  },
  {
    id: "running",
    label: "Running",
    description: "Pull requests with active analysis runs.",
  },
  {
    id: "blocked_stale",
    label: "Blocked or stale",
    description: "Pull requests blocked by stale run or draft context.",
  },
  {
    id: "published",
    label: "Published",
    description: "Pull requests with a submitted ReviewDesk review.",
  },
];

export function groupRunCentricQueue(items: RunCentricQueueItem[]): RunCentricQueueSection[] {
  const grouped = new Map<RunCentricQueueSectionId, RunCentricQueueItem[]>(
    RUN_CENTRIC_SECTIONS.map((section) => [section.id, []]),
  );

  for (const item of items) {
    grouped.get(queueSectionForItem(item))?.push(item);
  }

  return RUN_CENTRIC_SECTIONS.map((section) => ({
    ...section,
    items: grouped.get(section.id) ?? [],
  }));
}

export function isDraftStale(
  draft: Pick<ReviewDraftView, "base_head_sha" | "base_diff_hash">,
  current_head_sha: string,
  current_diff_hash: string,
): boolean {
  return draft.base_head_sha !== current_head_sha || draft.base_diff_hash !== current_diff_hash;
}

function queueSectionForItem(item: RunCentricQueueItem): RunCentricQueueSectionId {
  if (item.published_review_id != null) return "published";
  if (isStaleQueueItem(item)) return "blocked_stale";
  if (item.latest_run_status === "queued" || item.latest_run_status === "running") return "running";
  if (item.active_draft_id || item.latest_run_status === "draft_ready") return "draft_ready";
  return "needs_review";
}

function isStaleQueueItem(item: RunCentricQueueItem): boolean {
  return (
    item.active_draft_stale === true ||
    item.latest_run_status === "stale" ||
    Boolean(item.stale_status && item.stale_status !== "fresh")
  );
}

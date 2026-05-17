import {
  hasInvalidSelectedMapping,
  isPublishableInlineComment,
} from "./inline-comments";
import type {
  InlineCommentDraftView,
  InlineMappingStatus,
  ReviewEvent,
  ReviewPublishPayloadView,
} from "./workspace-view-models";

export type PublishBlocker =
  | "github_auth_required"
  | "github_scope_insufficient"
  | "github_sso_required"
  | "pr_closed"
  | "pr_merged"
  | "head_changed"
  | "diff_changed"
  | "inline_mapping_invalid"
  | "payload_empty"
  | "explicit_verdict_confirmation_required"
  | "private_diff_consent_required";

export type BuildPublishInlineCommentInput = Partial<InlineCommentDraftView> &
  Pick<InlineCommentDraftView, "id" | "body" | "selected_for_publish" | "dismissed"> & {
    mapping_status: InlineMappingStatus;
  };

export interface BuildPublishPayloadInput {
  owner: string;
  repo: string;
  number: number;
  expected_head_sha: string;
  expected_diff_hash: string;
  event: ReviewEvent;
  body: string;
  inline_comments: BuildPublishInlineCommentInput[];
  explicit_verdict_confirmed: boolean;
  private_diff_consent_required: boolean;
  private_diff_consent_accepted: boolean;
}

export interface PublishSafetyContext {
  current_head_sha: string;
  current_diff_hash: string;
  github_connected?: boolean;
  write_scope_valid?: boolean;
  sso_required?: boolean;
  pr_open?: boolean;
  pr_merged?: boolean;
}

export function buildPublishPayload(input: BuildPublishPayloadInput): ReviewPublishPayloadView {
  return {
    owner: input.owner,
    repo: input.repo,
    number: input.number,
    expected_head_sha: input.expected_head_sha,
    expected_diff_hash: input.expected_diff_hash,
    event: input.event,
    body: input.body,
    inline_comments: input.inline_comments.map(normalizeInlineComment),
    explicit_verdict_confirmed: input.explicit_verdict_confirmed,
    private_diff_consent_required: input.private_diff_consent_required,
    private_diff_consent_accepted: input.private_diff_consent_accepted,
  };
}

export function derivePublishBlockers(
  payload: ReviewPublishPayloadView,
  context: PublishSafetyContext,
): PublishBlocker[] {
  const blockers: PublishBlocker[] = [];

  if (context.github_connected === false) blockers.push("github_auth_required");
  if (context.write_scope_valid === false) blockers.push("github_scope_insufficient");
  if (context.sso_required === true) blockers.push("github_sso_required");
  if (context.pr_open === false) blockers.push("pr_closed");
  if (context.pr_merged === true) blockers.push("pr_merged");
  if (payload.expected_head_sha !== context.current_head_sha) blockers.push("head_changed");
  if (payload.expected_diff_hash !== context.current_diff_hash) blockers.push("diff_changed");
  if (payload.inline_comments.some(hasInvalidSelectedMapping)) blockers.push("inline_mapping_invalid");
  if (payload.body.trim().length === 0 && !payload.inline_comments.some(isPublishableInlineComment)) {
    blockers.push("payload_empty");
  }
  if (payload.event !== "COMMENT" && !payload.explicit_verdict_confirmed) {
    blockers.push("explicit_verdict_confirmation_required");
  }
  if (payload.private_diff_consent_required && !payload.private_diff_consent_accepted) {
    blockers.push("private_diff_consent_required");
  }

  return blockers;
}

function normalizeInlineComment(comment: BuildPublishInlineCommentInput): InlineCommentDraftView {
  return {
    id: comment.id,
    path: comment.path ?? "",
    side: comment.side ?? "RIGHT",
    line: comment.line ?? 0,
    start_line: comment.start_line ?? null,
    start_side: comment.start_side ?? null,
    body: comment.body,
    severity: comment.severity ?? null,
    confidence: comment.confidence ?? null,
    source_run_id: comment.source_run_id ?? null,
    source_finding_id: comment.source_finding_id ?? null,
    selected_for_publish: comment.selected_for_publish,
    dismissed: comment.dismissed,
    user_edited: comment.user_edited ?? false,
    mapping_status: comment.mapping_status,
  };
}

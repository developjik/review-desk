import { describe, expect, it } from "vitest";
import { initialWorkspaceState, workspaceReducer } from "./workspace-state";
import type { PullRequestQueueItem } from "./view-models";
import type { AnalysisRunView, ReviewDraftView } from "./workspace-view-models";

function queueItem(overrides: Partial<PullRequestQueueItem> = {}): PullRequestQueueItem {
  return {
    owner: "company",
    repo: "payment-web",
    number: 582,
    title: "Payment refactor",
    author: "alice",
    url: "https://github.com/company/payment-web/pull/582",
    draft: false,
    review_requested: true,
    assigned: true,
    ci_status: null,
    changed_files_count: 1,
    updated_at: "2026-05-17T00:00:00Z",
    ...overrides,
  };
}

function analysisRun(overrides: Partial<AnalysisRunView> = {}): AnalysisRunView {
  return {
    run_id: "run-a",
    owner: "company",
    repo: "payment-web",
    number: 582,
    head_sha: "head",
    diff_hash: "diff",
    context_hash: "context",
    mode: "fast",
    model_id: "gpt-5.5",
    reasoning_effort: "medium",
    review_language: "en",
    custom_prompt: null,
    prompt_version: "reviewdesk-run-centric-v1",
    selected_files: [],
    excluded_files: [],
    private_diff_consent_snapshot: true,
    status: "draft_ready",
    blocked_reason: null,
    draft_seed_body: "Generated draft",
    findings: [],
    created_at: "2026-05-17T00:00:00Z",
    completed_at: "2026-05-17T00:00:01Z",
    ...overrides,
  };
}

function reviewDraft(overrides: Partial<ReviewDraftView> = {}): ReviewDraftView {
  return {
    draft_id: "draft-a",
    owner: "company",
    repo: "payment-web",
    number: 582,
    source_run_ids: ["run-a"],
    base_head_sha: "head",
    base_diff_hash: "diff",
    verdict: "COMMENT",
    body: "Generated draft",
    inline_comments: [],
    user_edited: false,
    stale: false,
    created_at: "2026-05-17T00:00:00Z",
    updated_at: "2026-05-17T00:00:01Z",
    ...overrides,
  };
}

describe("workspace reducer", () => {
  it("ignores completed runs from a different selected PR", () => {
    const selectedPr = queueItem({ repo: "design-system", number: 41 });
    const existingDraft = reviewDraft({
      draft_id: "draft-b",
      repo: "design-system",
      number: 41,
      body: "Current PR draft",
    });
    const state = {
      ...initialWorkspaceState,
      selectedPr,
      activeDraft: existingDraft,
      runs: [],
    };

    const next = workspaceReducer(state, {
      type: "run_completed",
      run: analysisRun({ repo: "payment-web", number: 582 }),
      draft: reviewDraft({ body: "Stale PR draft" }),
    });

    expect(next).toBe(state);
  });
});

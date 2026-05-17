import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import * as ipc from "./lib/ipc";
import { sampleConnectedStatus, sampleContext, sampleQueue, sampleRepositories, sampleStatus } from "./lib/view-models";
import type { AnalysisRunView, ReviewDraftView } from "./lib/workspace-view-models";

vi.mock("./lib/ipc", () => ({
  getAppStatus: vi.fn(),
  getCodexBridgeStatus: vi.fn().mockResolvedValue(null),
  listAiModels: vi.fn().mockResolvedValue({ status: "blocked", models: [], disabled_reason: "model_list_unavailable" }),
  listRepositories: vi.fn(),
  loadReviewQueue: vi.fn(),
  collectPrContext: vi.fn(),
  listAnalysisRuns: vi.fn(),
  listReviewDrafts: vi.fn().mockResolvedValue([]),
  readReviewDraft: vi.fn(),
  openExternalUrl: vi.fn(),
  pollCodexChatGptLogin: vi.fn(),
  pollGithubOAuth: vi.fn(),
  prepareSubmitReview: vi.fn().mockResolvedValue({ status: "blocked", blocked_reasons: ["body_empty"], confirmation_id: null }),
  confirmSubmitReview: vi.fn(),
  refreshAiAccountStatus: vi.fn(),
  refreshGithubAuthStatus: vi.fn(),
  startAgentRun: vi.fn(),
  startCodexChatGptLogin: vi.fn(),
  startGithubOAuth: vi.fn(),
}));

function sampleAnalysisRun(overrides: Partial<AnalysisRunView> = {}): AnalysisRunView {
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
    selected_files: ["src/lib.rs"],
    excluded_files: [],
    private_diff_consent_snapshot: true,
    status: "draft_ready",
    blocked_reason: null,
    draft_seed_body: "AI draft",
    findings: [],
    created_at: "2026-05-17T00:00:00Z",
    completed_at: "2026-05-17T00:00:01Z",
    ...overrides,
  };
}

function sampleReviewDraft(overrides: Partial<ReviewDraftView> = {}): ReviewDraftView {
  return {
    draft_id: "draft-a",
    owner: "company",
    repo: "payment-web",
    number: 582,
    source_run_ids: ["run-a"],
    base_head_sha: "head",
    base_diff_hash: "diff",
    verdict: "COMMENT",
    body: "User edited draft",
    inline_comments: [],
    user_edited: true,
    stale: false,
    created_at: "2026-05-17T00:00:00Z",
    updated_at: "2026-05-17T00:00:02Z",
    ...overrides,
  };
}

describe("ReviewDesk app shell", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.listAnalysisRuns).mockResolvedValue([]);
    vi.mocked(ipc.listReviewDrafts).mockResolvedValue([]);
    vi.mocked(ipc.readReviewDraft).mockRejectedValue(new Error("draft_not_found"));
  });

  afterEach(() => {
    cleanup();
  });

  it("shows setup gate when GitHub is missing", async () => {
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleStatus());

    render(<App />);

    expect(await screen.findByText("Connect your review workspace")).toBeTruthy();
    expect(screen.getByText("Connect GitHub in Browser")).toBeTruthy();
  });

  it("shows PR-centric review shell when GitHub is connected", async () => {
    const queue = sampleQueue();
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));

    render(<App />);

    expect(await screen.findByText("Needs review")).toBeTruthy();
    await waitFor(() => expect(screen.getAllByText("Payment refactor with retry-safe session handling").length).toBeGreaterThan(0));
    expect(await screen.findByText("Review rail")).toBeTruthy();
    expect(screen.queryByText("Agent Runs")).toBeNull();
  });

  it("shows multiple runs without overwriting the active draft", async () => {
    const queue = sampleQueue();
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));
    vi.mocked(ipc.listAnalysisRuns).mockResolvedValue([
      sampleAnalysisRun({ run_id: "run-a", mode: "fast" }),
      sampleAnalysisRun({ run_id: "run-b", mode: "deep" }),
    ]);
    const activeDraft = sampleReviewDraft({ draft_id: "draft-a", body: "User edited draft" });
    vi.mocked(ipc.listReviewDrafts).mockResolvedValue([activeDraft]);
    vi.mocked(ipc.readReviewDraft).mockResolvedValue(activeDraft);

    render(<App />);

    expect(await screen.findByText("Run history")).toBeTruthy();
    expect(await screen.findByText("run-a")).toBeTruthy();
    expect(await screen.findByText("run-b")).toBeTruthy();
    expect(await screen.findByDisplayValue("User edited draft")).toBeTruthy();
  });

  it("loads the active workspace draft from listed drafts without reading a literal active id", async () => {
    const queue = sampleQueue();
    const recentDraft = sampleReviewDraft({
      draft_id: "draft-recent",
      body: "Persisted active draft",
      updated_at: "2026-05-17T00:00:03Z",
    });
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));
    vi.mocked(ipc.listAnalysisRuns).mockResolvedValue([]);
    vi.mocked(ipc.listReviewDrafts).mockResolvedValue([
      sampleReviewDraft({ draft_id: "draft-old", body: "Old draft", updated_at: "2026-05-17T00:00:01Z" }),
      recentDraft,
    ]);
    vi.mocked(ipc.readReviewDraft).mockImplementation(async ({ draft_id }) => {
      if (draft_id === "active") throw new Error("literal active id should not be read");
      return recentDraft;
    });

    render(<App />);

    expect(await screen.findByDisplayValue("Persisted active draft")).toBeTruthy();
    expect(ipc.readReviewDraft).toHaveBeenCalledWith({
      owner: queue[0].owner,
      repo: queue[0].repo,
      number: queue[0].number,
      draft_id: "draft-recent",
    });
    expect(ipc.readReviewDraft).not.toHaveBeenCalledWith(expect.objectContaining({ draft_id: "active" }));
  });
});

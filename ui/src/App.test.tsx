import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
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
  saveReviewDraft: vi.fn(),
  readReviewDraft: vi.fn(),
  validateInlineComments: vi.fn(),
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

function sampleInlineComment(
  overrides: Partial<ReviewDraftView["inline_comments"][number]> = {},
): ReviewDraftView["inline_comments"][number] {
  return {
    id: "inline-a",
    path: "src/lib.rs",
    side: "RIGHT",
    line: 1,
    start_line: null,
    start_side: null,
    body: "Inline body",
    severity: null,
    confidence: null,
    source_run_id: null,
    source_finding_id: null,
    selected_for_publish: true,
    dismissed: false,
    user_edited: false,
    mapping_status: "valid",
    ...overrides,
  };
}

describe("ReviewDesk app shell", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.listAnalysisRuns).mockResolvedValue([]);
    vi.mocked(ipc.saveReviewDraft).mockImplementation(({ draft }) => Promise.resolve(draft));
    vi.mocked(ipc.readReviewDraft).mockRejectedValue(new Error("draft_not_found"));
    vi.mocked(ipc.validateInlineComments).mockImplementation(({ comments }) => Promise.resolve(comments));
    vi.mocked(ipc.startAgentRun).mockResolvedValue({
      status: "draft_ready",
      body: "AI draft",
      report_path: null,
      disabled_reason: null,
      blocked_reason: null,
      run: null,
    });
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
    vi.mocked(ipc.readReviewDraft).mockResolvedValue(activeDraft);

    render(<App />);

    expect(await screen.findByText("Run history")).toBeTruthy();
    expect(await screen.findByText("run-a")).toBeTruthy();
    expect(await screen.findByText("run-b")).toBeTruthy();
    expect(await screen.findByDisplayValue("User edited draft")).toBeTruthy();
  });

  it("loads the active workspace draft through the active draft lookup", async () => {
    const queue = sampleQueue();
    const activeDraft = sampleReviewDraft({
      draft_id: "draft-active",
      body: "Persisted active draft",
    });
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));
    vi.mocked(ipc.listAnalysisRuns).mockResolvedValue([]);
    vi.mocked(ipc.readReviewDraft).mockResolvedValue(activeDraft);

    render(<App />);

    expect(await screen.findByDisplayValue("Persisted active draft")).toBeTruthy();
    expect(ipc.readReviewDraft).toHaveBeenCalledWith({
      owner: queue[0].owner,
      repo: queue[0].repo,
      number: queue[0].number,
      draft_id: "active",
    });
  });

  it("passes private diff consent into agent runs when accepted", async () => {
    const queue = sampleQueue();
    vi.mocked(ipc.getAppStatus).mockResolvedValue(aiReadyStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));

    render(<App />);

    fireEvent.click(await screen.findByRole("checkbox", { name: "Allow private diff analysis" }));
    fireEvent.click(screen.getByRole("button", { name: "Fast" }));

    await waitFor(() =>
      expect(ipc.startAgentRun).toHaveBeenCalledWith(
        expect.objectContaining({
          mode: "fast",
          diff_hash: sampleContext(queue[0]).diff_hash,
          context_hash: sampleContext(queue[0]).context_hash,
          private_diff_consent_required: true,
          private_diff_consent_accepted: true,
        }),
      ),
    );
  });

  it("persists generated legacy agent drafts as the active workspace draft", async () => {
    const queue = sampleQueue();
    vi.mocked(ipc.getAppStatus).mockResolvedValue(aiReadyStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));

    render(<App />);

    fireEvent.click(await screen.findByRole("checkbox", { name: "Allow private diff analysis" }));
    fireEvent.click(screen.getByRole("button", { name: "Fast" }));

    await waitFor(() =>
      expect(ipc.saveReviewDraft).toHaveBeenCalledWith({
        draft: expect.objectContaining({
          owner: queue[0].owner,
          repo: queue[0].repo,
          number: queue[0].number,
          body: "AI draft",
          base_head_sha: sampleContext(queue[0]).pr.head_sha,
          base_diff_hash: sampleContext(queue[0]).diff_hash,
          source_run_ids: expect.arrayContaining([expect.stringMatching(/^legacy-/)]),
        }),
        mark_active: true,
      }),
    );
  });

  it("persists draft body verdict and inline selection edits as the active draft", async () => {
    const queue = sampleQueue();
    const activeDraft = sampleReviewDraft({
      body: "Persisted body",
      inline_comments: [sampleInlineComment({ id: "inline-a", selected_for_publish: false })],
    });
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));
    vi.mocked(ipc.readReviewDraft).mockResolvedValue(activeDraft);

    render(<App />);

    const body = await screen.findByDisplayValue("Persisted body");
    fireEvent.change(body, { target: { value: "Edited body" } });
    fireEvent.click(screen.getByRole("button", { name: "APPROVE" }));
    fireEvent.click(screen.getByRole("button", { name: "Inline" }));
    fireEvent.click(await screen.findByRole("checkbox", { name: "Publish" }));

    await waitFor(() =>
      expect(ipc.saveReviewDraft).toHaveBeenCalledWith({
        draft: expect.objectContaining({
          draft_id: "draft-a",
          body: "Edited body",
          user_edited: true,
        }),
        mark_active: true,
      }),
    );
    await waitFor(() =>
      expect(ipc.saveReviewDraft).toHaveBeenCalledWith({
        draft: expect.objectContaining({
          verdict: "APPROVE",
          user_edited: true,
        }),
        mark_active: true,
      }),
    );
    await waitFor(() =>
      expect(ipc.saveReviewDraft).toHaveBeenCalledWith({
        draft: expect.objectContaining({
          inline_comments: [expect.objectContaining({ id: "inline-a", selected_for_publish: true })],
        }),
        mark_active: true,
      }),
    );
  });

  it("uses active draft provenance and derived GitHub auth state when preparing publish", async () => {
    const queue = sampleQueue();
    const context = {
      ...sampleContext(queue[0]),
      pr: { ...sampleContext(queue[0]).pr, head_sha: "current-head" },
      diff_hash: "current-diff",
    };
    vi.mocked(ipc.getAppStatus).mockResolvedValue({ ...sampleConnectedStatus(), github: "scope_missing" });
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(context);
    vi.mocked(ipc.readReviewDraft).mockResolvedValue(
      sampleReviewDraft({
        base_head_sha: "draft-head",
        base_diff_hash: "draft-diff",
        body: "Ready to publish",
      }),
    );

    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Safety" }));
    fireEvent.click(screen.getByRole("button", { name: "Prepare" }));

    await waitFor(() =>
      expect(ipc.prepareSubmitReview).toHaveBeenCalledWith(
        expect.objectContaining({
          github_connected: true,
          write_scope_valid: false,
          sso_required: false,
          current_head_sha: "current-head",
          current_diff_hash: "current-diff",
          payload: expect.objectContaining({
            expected_head_sha: "draft-head",
            expected_diff_hash: "draft-diff",
          }),
        }),
      ),
    );
  });

  it("validates inline comments before prepare and saves validation state", async () => {
    const queue = sampleQueue();
    const invalidComment = sampleInlineComment({ id: "inline-a", mapping_status: "invalid_line" });
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockResolvedValue(sampleContext(queue[0]));
    vi.mocked(ipc.readReviewDraft).mockResolvedValue(
      sampleReviewDraft({
        body: "Ready to publish",
        inline_comments: [sampleInlineComment({ id: "inline-a", mapping_status: "valid" })],
      }),
    );
    vi.mocked(ipc.validateInlineComments).mockResolvedValue([invalidComment]);

    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Safety" }));
    fireEvent.click(screen.getByRole("button", { name: "Prepare" }));

    await waitFor(() =>
      expect(ipc.validateInlineComments).toHaveBeenCalledWith(
        expect.objectContaining({
          diff_hash: sampleContext(queue[0]).diff_hash,
        }),
      ),
    );
    await waitFor(() =>
      expect(ipc.prepareSubmitReview).toHaveBeenCalledWith(
        expect.objectContaining({
          payload: expect.objectContaining({
            inline_comments: [expect.objectContaining({ id: "inline-a", mapping_status: "invalid_line" })],
          }),
        }),
      ),
    );
    expect(ipc.saveReviewDraft).toHaveBeenCalledWith({
      draft: expect.objectContaining({
        inline_comments: [expect.objectContaining({ id: "inline-a", mapping_status: "invalid_line" })],
      }),
      mark_active: true,
    });
  });

  it("ignores stale inline validation during prepare after switching PRs", async () => {
    const queue = sampleQueue();
    const validation = deferred<ReviewDraftView["inline_comments"]>();
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockImplementation((item) => Promise.resolve(sampleContext(item)));
    vi.mocked(ipc.readReviewDraft).mockImplementation(({ repo }) =>
      Promise.resolve(
        sampleReviewDraft({
          draft_id: `draft-${repo}`,
          owner: "company",
          repo,
          number: repo === queue[0].repo ? queue[0].number : queue[1].number,
          body: `Ready to publish ${repo}`,
          inline_comments: [sampleInlineComment({ id: `inline-${repo}`, mapping_status: "valid" })],
        }),
      ),
    );
    vi.mocked(ipc.validateInlineComments).mockReturnValue(validation.promise);

    render(<App />);

    expect(await screen.findByDisplayValue("Ready to publish payment-web")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Safety" }));
    fireEvent.click(screen.getByRole("button", { name: "Prepare" }));
    await waitFor(() => expect(ipc.validateInlineComments).toHaveBeenCalled());

    fireEvent.click(await findQueueItem("User hook refactor"));
    await waitFor(() => expect(screen.getAllByText("company/admin#588").length).toBeGreaterThan(0));

    validation.resolve([sampleInlineComment({ id: "inline-payment-web", mapping_status: "invalid_line" })]);
    await waitFor(() => expect(ipc.validateInlineComments).toHaveBeenCalledTimes(1));
    await Promise.resolve();
    await Promise.resolve();

    expect(ipc.prepareSubmitReview).not.toHaveBeenCalled();
    expect(screen.queryByText("Review body is empty")).toBeNull();
    expect(ipc.saveReviewDraft).not.toHaveBeenCalledWith({
      draft: expect.objectContaining({
        repo: "payment-web",
        inline_comments: [expect.objectContaining({ mapping_status: "invalid_line" })],
      }),
      mark_active: true,
    });
  });

  it("clears stale submit progress after switching PRs during confirmation", async () => {
    const queue = sampleQueue();
    const confirmation = deferred<Awaited<ReturnType<typeof ipc.confirmSubmitReview>>>();
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockImplementation((item) => Promise.resolve(sampleContext(item)));
    vi.mocked(ipc.readReviewDraft).mockImplementation(({ repo }) =>
      Promise.resolve(
        sampleReviewDraft({
          draft_id: `draft-${repo}`,
          owner: "company",
          repo,
          number: repo === queue[0].repo ? queue[0].number : queue[1].number,
          body: `Ready to publish ${repo}`,
          inline_comments: [],
        }),
      ),
    );
    vi.mocked(ipc.prepareSubmitReview).mockResolvedValue({
      status: "ready",
      blocked_reasons: [],
      confirmation_id: "confirm-a",
    });
    vi.mocked(ipc.confirmSubmitReview).mockReturnValue(confirmation.promise);

    render(<App />);

    expect(await screen.findByDisplayValue("Ready to publish payment-web")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Safety" }));
    fireEvent.click(screen.getByRole("button", { name: "Prepare" }));
    await waitFor(() => {
      const confirmButton = screen.getByRole("button", { name: "Confirm Submit" }) as HTMLButtonElement;
      expect(confirmButton.disabled).toBe(false);
    });
    fireEvent.click(screen.getByRole("button", { name: "Confirm Submit" }));
    fireEvent.click(screen.getByRole("button", { name: "Publish" }));
    await waitFor(() => expect(screen.getByText("Submitting review...")).toBeTruthy());

    fireEvent.click(await findQueueItem("User hook refactor"));
    await waitFor(() => expect(screen.getAllByText("company/admin#588").length).toBeGreaterThan(0));

    confirmation.resolve({ id: 991, url: "https://github.com/company/payment-web/pull/582#pullrequestreview-991" });
    await waitFor(() => expect(ipc.confirmSubmitReview).toHaveBeenCalledTimes(1));
    await Promise.resolve();
    await Promise.resolve();

    expect(screen.queryByText("Submitting review...")).toBeNull();
    expect(screen.queryByText("Submitted review #991")).toBeNull();
    expect(screen.queryByRole("button", { name: "Publish" })).toBeNull();
  });

  it("ignores slower PR selection responses after a newer PR is selected", async () => {
    const queue = sampleQueue();
    const firstContext = deferred<ReturnType<typeof sampleContext>>();
    const secondContext = {
      ...sampleContext(queue[1]),
      pr: { ...sampleContext(queue[1]).pr, head_sha: "second-head-sha" },
    };
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockImplementation((item) => {
      if (item.number === queue[0].number) return firstContext.promise;
      return Promise.resolve(secondContext);
    });

    render(<App />);

    fireEvent.click(await findQueueItem("User hook refactor"));
    expect(await screen.findByText("head second-h")).toBeTruthy();

    firstContext.resolve({
      ...sampleContext(queue[0]),
      pr: { ...sampleContext(queue[0]).pr, head_sha: "first-head-sha" },
    });

    await waitFor(() => expect(screen.getByText("head second-h")).toBeTruthy());
    expect(screen.queryByText("head first-he")).toBeNull();
  });

  it("does not copy a previous PR draft into a new PR without a persisted draft", async () => {
    const queue = sampleQueue();
    vi.mocked(ipc.getAppStatus).mockResolvedValue(sampleConnectedStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockImplementation((item) => Promise.resolve(sampleContext(item)));
    vi.mocked(ipc.readReviewDraft).mockImplementation(({ repo }) => {
      if (repo === queue[0].repo) {
        return Promise.resolve(sampleReviewDraft({ body: "First PR draft" }));
      }
      return Promise.reject(new Error("draft_not_found"));
    });

    render(<App />);

    expect(await screen.findByDisplayValue("First PR draft")).toBeTruthy();
    fireEvent.click(await findQueueItem("User hook refactor"));

    await waitFor(() => expect(screen.queryByDisplayValue("First PR draft")).toBeNull());
  });

  it("does not apply an agent run completion after switching PRs", async () => {
    const queue = sampleQueue();
    const runCompletion = deferred<Awaited<ReturnType<typeof ipc.startAgentRun>>>();
    vi.mocked(ipc.getAppStatus).mockResolvedValue(aiReadyStatus());
    vi.mocked(ipc.listRepositories).mockResolvedValue(sampleRepositories());
    vi.mocked(ipc.loadReviewQueue).mockResolvedValue(queue);
    vi.mocked(ipc.collectPrContext).mockImplementation((item) => Promise.resolve(sampleContext(item)));
    vi.mocked(ipc.startAgentRun).mockReturnValue(runCompletion.promise);

    render(<App />);

    fireEvent.click(await screen.findByRole("checkbox", { name: "Allow private diff analysis" }));
    fireEvent.click(screen.getByRole("button", { name: "Fast" }));
    await waitFor(() => expect(ipc.startAgentRun).toHaveBeenCalled());

    fireEvent.click(await findQueueItem("User hook refactor"));
    await waitFor(() => expect(screen.getAllByText("company/admin#588").length).toBeGreaterThan(0));

    runCompletion.resolve({
      status: "draft_ready",
      body: "Stale AI draft",
      report_path: null,
      disabled_reason: null,
      blocked_reason: null,
      run: null,
    });

    await waitFor(() => expect(screen.getAllByText("company/admin#588").length).toBeGreaterThan(0));
    expect(screen.queryByDisplayValue("Stale AI draft")).toBeNull();
  });
});

function aiReadyStatus(): ReturnType<typeof sampleConnectedStatus> {
  const status = sampleConnectedStatus();
  return {
    ...status,
    chatgpt: "connected" as const,
    ai_review_available: true,
    capabilities: {
      ...status.capabilities!,
      ai_review_available: true,
      blocked_reasons: [],
    },
    generation_available: true,
    generation_blocked_reason: null,
    ai_blocked_reason: null,
    ai_connection: status.ai_connection
      ? {
          ...status.ai_connection,
          status: "connected" as const,
          blockedReason: null,
        }
      : status.ai_connection,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

async function findQueueItem(title: string) {
  const [itemTitle] = await screen.findAllByText(title);
  return itemTitle.closest("button") ?? itemTitle;
}

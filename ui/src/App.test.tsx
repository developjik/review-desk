import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import * as ipc from "./lib/ipc";
import { sampleConnectedStatus, sampleContext, sampleQueue, sampleRepositories, sampleStatus } from "./lib/view-models";

vi.mock("./lib/ipc", () => ({
  getAppStatus: vi.fn(),
  getCodexBridgeStatus: vi.fn().mockResolvedValue(null),
  listAiModels: vi.fn().mockResolvedValue({ status: "blocked", models: [], disabled_reason: "model_list_unavailable" }),
  listRepositories: vi.fn(),
  loadReviewQueue: vi.fn(),
  collectPrContext: vi.fn(),
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

describe("ReviewDesk app shell", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
});

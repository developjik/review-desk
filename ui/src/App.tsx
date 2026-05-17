import { Bot, Brain, Command, Github, Languages, Search, Settings } from "lucide-react";
import { useEffect, useMemo, useReducer, useRef, useState } from "react";
import { AppShell } from "./components/AppShell";
import { Badge, Button, Kbd, Panel } from "./components/ui";
import {
  collectPrContext,
  confirmSubmitReview,
  getAppStatus,
  getCodexBridgeStatus,
  listAiModels,
  listAnalysisRuns,
  listReviewDrafts,
  listRepositories,
  loadReviewQueue,
  openExternalUrl,
  pollCodexChatGptLogin,
  pollGithubOAuth,
  prepareSubmitReview,
  readReviewDraft,
  refreshAiAccountStatus,
  refreshGithubAuthStatus,
  startAgentRun,
  startCodexChatGptLogin,
  startGithubOAuth,
  type ReviewPublishPayload,
  type SubmitPreflightView,
} from "./lib/ipc";
import { t } from "./lib/i18n";
import { dismissInlineCommentById, setInlineCommentSelection } from "./lib/inline-comments";
import { initialWorkspaceState, workspaceReducer } from "./lib/workspace-state";
import {
  commandDisabledReason,
  defaultLanguagePreferences,
  deriveSubmitSafetyState,
  groupQueueBySection,
  pullRequestRef,
  reviewSearchPlaceholder,
  sampleStatus,
  shouldShowStartupGate,
  type AgentRunRecordView,
  type AppStatusView,
  type ChangedFileContext,
  type CodexBridgeStatusView,
  type LanguagePreferences,
  type Locale,
  type PullRequestContextView,
  type PullRequestQueueItem,
  type ReasoningEffort,
  type Repository,
} from "./lib/view-models";
import type { AnalysisRunMode, AnalysisRunView, InlineCommentDraftView, ReviewDraftView } from "./lib/workspace-view-models";
import {
  authPollTerminalMessage,
  shouldPollCodexLogin,
  shouldPollGithubOAuth,
} from "./lib/auth-flow";

const verdicts = ["COMMENT", "APPROVE", "REQUEST_CHANGES"] as const;
const appScreens = ["inbox", "settings"] as const;
const authPollMaxAttempts = 90;

type Verdict = (typeof verdicts)[number];
type Screen = "setup" | (typeof appScreens)[number];
type LoadState = "idle" | "loading" | "ready" | "error";

export default function App() {
  const commandInputRef = useRef<HTMLInputElement>(null);
  const [workspace, dispatchWorkspace] = useReducer(workspaceReducer, initialWorkspaceState);
  const [status, setStatus] = useState<AppStatusView>(sampleStatus());
  const [languagePreferences, setLanguagePreferences] = useState<LanguagePreferences>(
    sampleStatus().language_preferences ?? defaultLanguagePreferences(navigator.language),
  );
  const [activeScreen, setActiveScreen] = useState<Screen>("setup");
  const [repositories, setRepositories] = useState<Repository[]>([]);
  const [queue, setQueue] = useState<PullRequestQueueItem[]>([]);
  const [queueState, setQueueState] = useState<LoadState>("idle");
  const [queueError, setQueueError] = useState<string | null>(null);
  const [selectedRepo, setSelectedRepo] = useState<string | null>(null);
  const [selectedPr, setSelectedPr] = useState<PullRequestQueueItem | null>(null);
  const [context, setContext] = useState<PullRequestContextView | null>(null);
  const [contextState, setContextState] = useState<LoadState>("idle");
  const [contextError, setContextError] = useState<string | null>(null);
  const [selectedFile, setSelectedFile] = useState<ChangedFileContext | null>(null);
  const [commandQuery, setCommandQuery] = useState("");
  const [filter, setFilter] = useState<"all" | "requested" | "assigned" | "ci_failed">("all");
  const [viewedFiles, setViewedFiles] = useState<Set<string>>(new Set());
  const [submittedRefs, setSubmittedRefs] = useState<Set<string>>(new Set());
  const [agentStatus, setAgentStatus] = useState("blocked");
  const [agentMessage, setAgentMessage] = useState("codex_chatgpt_auth_required");
  const [model, setModel] = useState("gpt-5.5");
  const [reasoningDepth, setReasoningDepth] = useState<ReasoningEffort>("medium");
  const [draft, setDraft] = useState("");
  const [verdict, setVerdict] = useState<Verdict>("COMMENT");
  const [explicitVerdict, setExplicitVerdict] = useState(false);
  const [privateConsent, setPrivateConsent] = useState(false);
  const [preflight, setPreflight] = useState<SubmitPreflightView | null>(null);
  const [draftDirtySincePreflight, setDraftDirtySincePreflight] = useState(true);
  const [submitMessage, setSubmitMessage] = useState<string | null>(null);
  const [submittedReviewId, setSubmittedReviewId] = useState<number | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [oauthMessage, setOauthMessage] = useState<string | null>(null);
  const [codexBridgeStatus, setCodexBridgeStatus] = useState<CodexBridgeStatusView | null>(null);

  const locale = languagePreferences.ui_locale;
  const appShellAvailable = status.capabilities?.app_shell_available ?? status.github === "connected";
  const selectedRef = selectedPr ? pullRequestRef(selectedPr) : "No PR selected";
  const activeDraft = workspace.activeDraft;
  const activeDraftBody = activeDraft?.body ?? draft;

  const filteredQueue = useMemo(() => {
    const query = commandQuery.trim().toLowerCase();
    return queue.filter((item) => {
      const matchesFilter =
        filter === "all" ||
        (filter === "requested" && item.review_requested) ||
        (filter === "assigned" && item.assigned) ||
        (filter === "ci_failed" && item.ci_status === "fail");
      const matchesQuery =
        query.length === 0 ||
        item.title.toLowerCase().includes(query) ||
        item.repo.toLowerCase().includes(query) ||
        item.owner.toLowerCase().includes(query) ||
        String(item.number).includes(query) ||
        context?.files.some((file) => file.path.toLowerCase().includes(query));
      return matchesFilter && matchesQuery;
    });
  }, [commandQuery, context?.files, filter, queue]);

  const draftRefs = useMemo(() => {
    const refs = new Set<string>();
    if (selectedPr && activeDraftBody.trim()) refs.add(pullRequestRef(selectedPr));
    return refs;
  }, [activeDraftBody, selectedPr]);

  const inboxSections = useMemo(
    () => groupQueueBySection(filteredQueue, { draftRefs, submittedRefs }),
    [draftRefs, filteredQueue, submittedRefs],
  );

  const safetyState = deriveSubmitSafetyState({
    hasTarget: Boolean(selectedPr && context),
    draftBody: activeDraftBody,
    draftDirtySincePreflight,
    preflight,
    submitting,
    submittedReviewId,
    submitMessage,
  });

  const publishPayload = useMemo(
    () => (selectedPr && context ? currentReviewPublishPayload(selectedPr, context) : null),
    [activeDraft, activeDraftBody, context, explicitVerdict, privateConsent, selectedPr, verdict],
  );

  useEffect(() => {
    getAppStatus()
      .then((nextStatus) => {
        setStatus(nextStatus);
        if (nextStatus.language_preferences) setLanguagePreferences(nextStatus.language_preferences);
        setModel(nextStatus.selected_model);
        setReasoningDepth(nextStatus.selected_reasoning_effort ?? (nextStatus.selected_reasoning_depth as ReasoningEffort));
        setActiveScreen(shouldShowStartupGate(nextStatus) ? "setup" : "inbox");
      })
      .catch(() => setStatus(sampleStatus()));
    getCodexBridgeStatus()
      .then(setCodexBridgeStatus)
      .catch(() => setCodexBridgeStatus(null));
  }, []);

  useEffect(() => {
    if (!appShellAvailable) return;
    listRepositories()
      .then(setRepositories)
      .catch(() => setRepositories([]));
    listAiModels()
      .then((modelList) => {
        if (modelList.models.length > 0) {
          setStatus((current) => ({ ...current, ai_models: modelList.models }));
        }
      })
      .catch(() => undefined);
  }, [appShellAvailable]);

  useEffect(() => {
    if (!appShellAvailable) return;
    void refreshQueue(true);
  }, [appShellAvailable, selectedRepo]);

  useEffect(() => {
    if (shouldShowStartupGate(status)) {
      setActiveScreen("setup");
    } else if (activeScreen === "setup") {
      setActiveScreen("inbox");
    }
  }, [activeScreen, status]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        commandInputRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    const refreshVisibleAuthState = () => {
      if (document.visibilityState === "visible") {
        void refreshStatus().catch(() => undefined);
      }
    };
    window.addEventListener("focus", refreshVisibleAuthState);
    document.addEventListener("visibilitychange", refreshVisibleAuthState);
    return () => {
      window.removeEventListener("focus", refreshVisibleAuthState);
      document.removeEventListener("visibilitychange", refreshVisibleAuthState);
    };
  }, []);

  async function refreshStatus() {
    const nextStatus = await getAppStatus();
    setStatus(nextStatus);
    if (nextStatus.language_preferences) setLanguagePreferences(nextStatus.language_preferences);
    setModel(nextStatus.selected_model);
    setReasoningDepth(nextStatus.selected_reasoning_effort ?? (nextStatus.selected_reasoning_depth as ReasoningEffort));
    if (!shouldShowStartupGate(nextStatus) && activeScreen === "setup") setActiveScreen("inbox");
  }

  async function refreshQueue(autoSelect = false) {
    setQueueState("loading");
    setQueueError(null);
    const items = await loadReviewQueue(selectedRepo).catch((error: unknown) => {
      setQueueState("error");
      setQueueError(error instanceof Error ? error.message : "review_queue_failed");
      return null;
    });
    if (!items) return;
    setQueue(items);
    setQueueState("ready");
    if (autoSelect && !selectedPr && items[0]) {
      void selectPullRequest(items[0]);
    }
  }

  async function selectPullRequest(item: PullRequestQueueItem) {
    setSelectedPr(item);
    dispatchWorkspace({ type: "select_pr", pr: item });
    setContextState("loading");
    setContextError(null);
    setPreflight(null);
    setDraftDirtySincePreflight(true);
    setSubmittedReviewId(null);
    const nextContext = await collectPrContext(item).catch((error: unknown) => {
      setContextState("error");
      setContextError(error instanceof Error ? error.message : "pr_context_failed");
      return null;
    });
    if (!nextContext) return;
    setContext(nextContext);
    setSelectedFile(nextContext.files[0] ?? null);
    setViewedFiles(new Set());
    setContextState("ready");

    const workspaceInput = { owner: item.owner, repo: item.repo, number: item.number };
    const [runsResult, draftResult] = await Promise.allSettled([
      listAnalysisRuns(workspaceInput),
      loadWorkspaceDraft(workspaceInput),
    ]);
    dispatchWorkspace({
      type: "load_runs_success",
      runs: runsResult.status === "fulfilled" && Array.isArray(runsResult.value) ? runsResult.value : [],
    });
    const fallbackDraft = createManualReviewDraft(item, nextContext, draft);
    const nextDraft = draftResult.status === "fulfilled" && draftResult.value ? draftResult.value : fallbackDraft;
    dispatchWorkspace({ type: "load_draft_success", draft: nextDraft });
    setDraft(nextDraft.body);
    setVerdict(nextDraft.verdict);
  }

  function setUiLocale(uiLocale: Locale) {
    setLanguagePreferences((current) => ({ ...current, ui_locale: uiLocale }));
  }

  function setReviewLocale(reviewLocale: Locale) {
    setLanguagePreferences((current) => ({ ...current, review_locale: reviewLocale }));
  }

  async function pollGithubUntilComplete(flowId: string, intervalSeconds: number | null) {
    let waitMs = Math.max(1000, (intervalSeconds ?? 2) * 1000);
    for (let attempt = 0; attempt < authPollMaxAttempts; attempt += 1) {
      await sleep(waitMs);
      const poll = await pollGithubOAuth(flowId).catch((error: unknown) => ({
        flow_id: flowId,
        mode: "browser",
        status: "failed",
        account_login: null,
        user_code: null,
        verification_uri: null,
        disabled_reason: error instanceof Error ? error.message : "github_poll_failed",
      }));
      const message = authPollTerminalMessage("GitHub", poll);
      if (message) setOauthMessage(message);
      if (poll.status === "authorized") {
        await refreshGithubAuthStatus().catch(() => null);
        await refreshStatus().catch(() => undefined);
        return;
      }
      if (poll.status === "slow_down") waitMs += 5000;
      if (["denied", "failed", "timeout", "cancelled", "blocked"].includes(poll.status)) return;
    }
    setOauthMessage("GitHub authorization timed out. Try connecting again.");
  }

  async function pollCodexUntilComplete(loginId: string) {
    for (let attempt = 0; attempt < authPollMaxAttempts; attempt += 1) {
      await sleep(2000);
      const poll = await pollCodexChatGptLogin(loginId).catch((error: unknown) => ({
        flow_id: loginId,
        mode: "codex_chatgpt",
        status: "failed",
        account_login: null,
        user_code: null,
        verification_uri: null,
        disabled_reason: error instanceof Error ? error.message : "codex_poll_failed",
      }));
      const message = authPollTerminalMessage("Codex", poll);
      if (message) setOauthMessage(message);
      if (poll.status === "authorized" || poll.status === "connected") {
        await refreshAiAccountStatus().catch(() => null);
        await refreshStatus().catch(() => undefined);
        return;
      }
      if (["denied", "failed", "timeout", "cancelled", "blocked"].includes(poll.status)) return;
    }
    setOauthMessage("Codex authorization timed out. Try connecting again.");
  }

  async function startOAuth(kind: "github" | "chatgpt", mode: "browser" | "device" = "browser") {
    if (kind === "chatgpt") {
      const result = await startCodexChatGptLogin(mode).catch((error: unknown) => ({
        login_id: "codex-chatgpt-unavailable",
        status: "blocked",
        mode,
        auth_url: null,
        verification_url: null,
        user_code: null,
        disabled_reason: error instanceof Error ? error.message : "codex_chatgpt_unsupported",
      }));
      const url = result.auth_url ?? result.verification_url;
      const details = result.user_code ? ` Code: ${result.user_code}` : "";
      if (url) {
        await openExternalUrl(url);
        setOauthMessage(`Codex ${result.status}. Browser opened.${details}`);
        if (shouldPollCodexLogin(result)) void pollCodexUntilComplete(result.login_id);
      } else {
        setOauthMessage(result.disabled_reason ?? `Codex ${result.status}.${details}`);
        if (result.status === "connected") await refreshStatus().catch(() => undefined);
      }
      return;
    }

    const result = await startGithubOAuth(mode).catch((error: unknown) => ({
      flow_id: "github-unavailable",
      status: "blocked",
      mode,
      auth_url: null,
      redirect_uri: null,
      user_code: null,
      verification_uri: null,
      expires_in: null,
      interval: null,
      disabled_reason: error instanceof Error ? error.message : "github_oauth_unavailable",
    }));
    const url = result.auth_url ?? result.verification_uri;
    const details = result.user_code ? ` Code: ${result.user_code}` : "";
    if (!url) {
      setOauthMessage(result.disabled_reason ?? `GitHub OAuth ${result.status}.${details}`);
      return;
    }
    await openExternalUrl(url);
    setOauthMessage(`GitHub OAuth ${result.status}. Browser opened.${details}`);
    if (shouldPollGithubOAuth(result)) void pollGithubUntilComplete(result.flow_id, result.interval);
  }

  async function runAgent(mode: AnalysisRunMode = "fast") {
    const disabled = commandDisabledReason(status, "generate_review_draft");
    if (disabled && disabled !== "generation_adapter_unavailable") {
      setAgentStatus("blocked");
      setAgentMessage(disabled);
      return;
    }
    if (!selectedPr || !context) {
      setAgentStatus("blocked");
      setAgentMessage("Select a PR before running agents.");
      return;
    }
    setAgentStatus("running");
    setAgentMessage(`Running ${mode} analysis with ${model} and ${reasoningDepth} reasoning in ${languagePreferences.review_locale}`);
    const result = await startAgentRun({
      owner: selectedPr.owner,
      repo: selectedPr.repo,
      number: selectedPr.number,
      files: context.files,
      model,
      reasoning_depth: reasoningDepth,
      reasoning_effort: reasoningDepth,
      review_language: languagePreferences.review_locale,
      head_sha: context.pr.head_sha,
      private_diff_consent_required: true,
      private_diff_consent_accepted: privateConsent,
    }).catch((error: unknown) => ({
      status: "failed",
      body: "",
      report_path: null,
      disabled_reason: error instanceof Error ? error.message : "generation_failed",
      blocked_reason: "generation_failed",
      run: null,
    }));
    setAgentStatus(result.status);
    setAgentMessage(result.blocked_reason ?? result.disabled_reason ?? result.report_path ?? "Draft ready");
    const completedRun = analysisRunFromAgentResult(result.run, selectedPr, context, mode, model, reasoningDepth, languagePreferences.review_locale, privateConsent, result.body);
    const generatedDraft = result.body.trim() ? reviewDraftFromRun(completedRun, result.body, activeDraft?.verdict ?? verdict) : null;
    dispatchWorkspace({ type: "run_completed", run: completedRun, draft: generatedDraft });
    if (result.body.trim()) {
      if (!activeDraft?.user_edited) {
        setDraft(result.body);
        setDraftDirtySincePreflight(true);
        setPreflight(null);
      }
    }
  }

  async function prepareSubmit() {
    if (!selectedPr || !context) return;
    setSubmitMessage(null);
    const payload = currentReviewPublishPayload(selectedPr, context);
    const next = await prepareSubmitReview({
      payload,
      github_connected: status.github === "connected",
      write_scope_valid: true,
      sso_required: false,
      pr_open: context.pr.state === "open",
      pr_merged: context.pr.merged,
      current_head_sha: context.pr.head_sha,
      current_diff_hash: context.diff_hash,
    });
    setPreflight(next);
    dispatchWorkspace({ type: "prepare_publish_success", preflight: next });
    setDraftDirtySincePreflight(false);
  }

  async function confirmSubmit() {
    if (!selectedPr || !context || preflight?.status !== "ready" || !preflight.confirmation_id) return;
    setSubmitting(true);
    setSubmitMessage("Submitting review...");
    try {
      const payload = currentReviewPublishPayload(selectedPr, context);
      const submitted = await confirmSubmitReview({
        payload,
        confirmation_id: preflight.confirmation_id,
      });
      setSubmittedReviewId(submitted.id);
      setSubmittedRefs((current) => new Set(current).add(pullRequestRef(selectedPr)));
      setSubmitMessage(`Submitted review #${submitted.id}`);
      setPreflight(null);
    } catch (error) {
      setSubmitMessage(error instanceof Error ? error.message : "submit_failed");
    } finally {
      setSubmitting(false);
    }
  }

  function updateDraft(next: string) {
    setDraft(next);
    dispatchWorkspace({ type: "edit_draft_body", body: next });
    setDraftDirtySincePreflight(true);
    setPreflight(null);
    setSubmitMessage(null);
    setSubmittedReviewId(null);
  }

  function currentReviewPublishPayload(
    target: PullRequestQueueItem,
    currentContext: PullRequestContextView,
  ): ReviewPublishPayload {
    return {
      owner: target.owner,
      repo: target.repo,
      number: target.number,
      expected_head_sha: currentContext.pr.head_sha,
      expected_diff_hash: currentContext.diff_hash,
      body: activeDraftBody,
      event: activeDraft?.verdict ?? verdict,
      inline_comments: activeDraft?.inline_comments ?? [],
      explicit_verdict_confirmed: explicitVerdict,
      private_diff_consent_required: agentStatus !== "blocked",
      private_diff_consent_accepted: privateConsent,
    };
  }

  function updateVerdict(next: Verdict) {
    setVerdict(next);
    if (activeDraft) {
      dispatchWorkspace({ type: "replace_draft_from_run", draft: { ...activeDraft, verdict: next, user_edited: true, updated_at: new Date().toISOString() } });
    }
    setExplicitVerdict(next === "COMMENT");
    setDraftDirtySincePreflight(true);
    setPreflight(null);
    setSubmitMessage(null);
    setSubmittedReviewId(null);
  }

  function replaceDraftFromRun(run: AnalysisRunView) {
    const body = run.draft_seed_body ?? "";
    const nextDraft = reviewDraftFromRun(run, body, activeDraft?.verdict ?? verdict);
    dispatchWorkspace({ type: "replace_draft_from_run", draft: nextDraft });
    setDraft(nextDraft.body);
    setVerdict(nextDraft.verdict);
    setDraftDirtySincePreflight(true);
    setPreflight(null);
    setSubmitMessage(null);
  }

  function updateInlineSelection(id: string, selected: boolean) {
    if (!activeDraft) return;
    const comments = setInlineCommentSelection(activeDraft.inline_comments, id, selected);
    dispatchWorkspace({ type: "validate_inline_success", comments });
    setDraftDirtySincePreflight(true);
    setPreflight(null);
  }

  function dismissInline(id: string) {
    if (!activeDraft) return;
    const comments = dismissInlineCommentById(activeDraft.inline_comments, id);
    dispatchWorkspace({ type: "validate_inline_success", comments });
    setDraftDirtySincePreflight(true);
    setPreflight(null);
  }

  if (shouldShowStartupGate(status)) {
    return (
      <main className="min-h-screen bg-zinc-950 text-zinc-100">
        <AuthSetupScreen
          locale={locale}
          status={status}
          languagePreferences={languagePreferences}
          oauthMessage={oauthMessage}
          onConnectGithub={() => startOAuth("github", "browser")}
          onConnectGithubDevice={() => startOAuth("github", "device")}
          onConnectChatGpt={() => startOAuth("chatgpt", "browser")}
          onConnectChatGptDevice={() => startOAuth("chatgpt", "device")}
          onContinueWithoutAi={() => setActiveScreen("inbox")}
          onSetUiLocale={setUiLocale}
          onSetReviewLocale={setReviewLocale}
        />
      </main>
    );
  }

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100">
      <AppShell
        topBar={
          <TopBar
            locale={locale}
            status={status}
            model={model}
            languagePreferences={languagePreferences}
            commandInputRef={commandInputRef}
            commandQuery={commandQuery}
            onCommandQueryChange={setCommandQuery}
          />
        }
        activeScreen={activeScreen === "settings" ? "settings" : "inbox"}
        navItems={appScreens.map((screen) => ({
          id: screen,
          label: screen === "inbox" ? t(locale, "nav.inbox") : t(locale, "nav.settings"),
        }))}
        onSetActiveScreen={setActiveScreen}
        settingsContent={
          <SettingsScreen
            locale={locale}
            status={status}
            codexBridgeStatus={codexBridgeStatus}
            languagePreferences={languagePreferences}
            onSetUiLocale={setUiLocale}
            onSetReviewLocale={setReviewLocale}
          />
        }
        locale={locale}
        queueProps={{
          repositories,
          selectedRepo,
          filter,
          queueState,
          queueError,
          sections: inboxSections,
          selectedPr,
          onSelectRepo: setSelectedRepo,
          onSetFilter: setFilter,
          onRefresh: () => refreshQueue(false),
          onSelectPr: selectPullRequest,
        }}
        workspaceProps={{
          selectedPr,
          context,
          contextState,
          contextError,
          selectedFile,
          viewedFiles,
          onSelectFile: setSelectedFile,
          onMarkViewed: (path) => setViewedFiles((current) => new Set(current).add(path)),
          onRetryContext: () => selectedPr && selectPullRequest(selectedPr),
        }}
        draftPublishProps={{
          selectedRef,
          draft: activeDraft,
          runs: workspace.runs,
          explicitVerdict,
          safetyState,
          preflight,
          submitMessage,
          publishPayload,
          submitting,
          runDisabled: !selectedPr || !context,
          onRunMode: runAgent,
          onRunCustom: () => runAgent("custom"),
          onReplaceDraftFromRun: replaceDraftFromRun,
          onSetVerdict: updateVerdict,
          onSetExplicitVerdict: (confirmed) => {
            setExplicitVerdict(confirmed);
            setDraftDirtySincePreflight(true);
            setPreflight(null);
          },
          onSetDraftBody: updateDraft,
          onToggleInlineSelected: updateInlineSelection,
          onDismissInline: dismissInline,
          onPrepare: prepareSubmit,
          onConfirm: confirmSubmit,
        }}
      />
      <div className="sr-only">
        <Search />
      </div>
    </main>
  );
}

function createManualReviewDraft(
  item: PullRequestQueueItem,
  context: PullRequestContextView,
  body: string,
): ReviewDraftView {
  const now = new Date().toISOString();
  return {
    draft_id: `manual-${item.owner}-${item.repo}-${item.number}`,
    owner: item.owner,
    repo: item.repo,
    number: item.number,
    source_run_ids: [],
    base_head_sha: context.pr.head_sha,
    base_diff_hash: context.diff_hash,
    verdict: "COMMENT",
    body,
    inline_comments: [],
    user_edited: body.trim().length > 0,
    stale: false,
    created_at: now,
    updated_at: now,
  };
}

async function loadWorkspaceDraft(input: {
  owner: string;
  repo: string;
  number: number;
}): Promise<ReviewDraftView | null> {
  const drafts = await listReviewDrafts(input).catch(() => []);
  const selectedDraft = selectMostRecentDraft(drafts);
  if (!selectedDraft) return null;
  return readReviewDraft({ ...input, draft_id: selectedDraft.draft_id }).catch(() => selectedDraft);
}

function selectMostRecentDraft(drafts: ReviewDraftView[]): ReviewDraftView | null {
  if (drafts.length === 0) return null;
  return [...drafts].sort((left, right) => right.updated_at.localeCompare(left.updated_at))[0] ?? null;
}

function analysisRunFromAgentResult(
  record: AgentRunRecordView | null,
  item: PullRequestQueueItem,
  context: PullRequestContextView,
  mode: AnalysisRunMode,
  model: string,
  reasoningEffort: ReasoningEffort,
  reviewLanguage: Locale,
  privateConsent: boolean,
  draftSeedBody: string,
): AnalysisRunView {
  const now = new Date().toISOString();
  if (record) {
    return {
      run_id: record.run_id,
      owner: item.owner,
      repo: item.repo,
      number: item.number,
      head_sha: record.head_sha,
      diff_hash: record.diff_hash,
      context_hash: context.context_hash,
      mode,
      model_id: record.model_id,
      reasoning_effort: record.reasoning_effort,
      review_language: record.review_language,
      custom_prompt: null,
      prompt_version: record.prompt_version,
      selected_files: record.selected_files,
      excluded_files: [],
      private_diff_consent_snapshot: record.private_diff_consent_snapshot,
      status: record.status === "blocked" ? "failed" : record.status,
      blocked_reason: record.blocked_reason ?? null,
      draft_seed_body: draftSeedBody.trim() ? draftSeedBody : null,
      findings: [],
      created_at: record.started_at,
      completed_at: record.completed_at ?? now,
    };
  }

  return {
    run_id: `legacy-${item.owner}-${item.repo}-${item.number}-${Date.now()}`,
    owner: item.owner,
    repo: item.repo,
    number: item.number,
    head_sha: context.pr.head_sha,
    diff_hash: context.diff_hash,
    context_hash: context.context_hash,
    mode,
    model_id: model,
    reasoning_effort: reasoningEffort,
    review_language: reviewLanguage,
    custom_prompt: null,
    prompt_version: "legacy-start-agent-run",
    selected_files: context.files.filter((file) => file.ai_included).map((file) => file.path),
    excluded_files: context.files.filter((file) => !file.ai_included).map((file) => file.path),
    private_diff_consent_snapshot: privateConsent,
    status: draftSeedBody.trim() ? "draft_ready" : "failed",
    blocked_reason: null,
    draft_seed_body: draftSeedBody.trim() ? draftSeedBody : null,
    findings: [],
    created_at: now,
    completed_at: now,
  };
}

function reviewDraftFromRun(
  run: AnalysisRunView,
  body: string,
  verdict: Verdict,
): ReviewDraftView {
  const now = new Date().toISOString();
  return {
    draft_id: `draft-${run.run_id}`,
    owner: run.owner,
    repo: run.repo,
    number: run.number,
    source_run_ids: [run.run_id],
    base_head_sha: run.head_sha,
    base_diff_hash: run.diff_hash,
    verdict,
    body,
    inline_comments: findingsToInlineComments(run),
    user_edited: false,
    stale: false,
    created_at: now,
    updated_at: now,
  };
}

function findingsToInlineComments(run: AnalysisRunView): InlineCommentDraftView[] {
  return run.findings
    .filter((finding) => finding.path && finding.line && finding.body)
    .map((finding, index) => ({
      id: finding.id ?? `${run.run_id}-finding-${index}`,
      path: finding.path ?? "",
      side: "RIGHT",
      line: finding.line ?? 0,
      start_line: null,
      start_side: null,
      body: finding.body ?? "",
      severity: finding.severity ?? null,
      confidence: finding.confidence ?? null,
      source_run_id: run.run_id,
      source_finding_id: finding.id ?? null,
      selected_for_publish: true,
      dismissed: false,
      user_edited: false,
      mapping_status: "valid",
    }));
}

function TopBar({
  locale,
  status,
  model,
  languagePreferences,
  commandInputRef,
  commandQuery,
  onCommandQueryChange,
}: {
  locale: Locale;
  status: AppStatusView;
  model: string;
  languagePreferences: LanguagePreferences;
  commandInputRef: React.RefObject<HTMLInputElement | null>;
  commandQuery: string;
  onCommandQueryChange: (value: string) => void;
}) {
  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b border-zinc-800 bg-zinc-950 px-4">
      <div className="flex h-9 min-w-0 flex-1 items-center gap-2 rounded-md border border-zinc-800 bg-zinc-900 px-3">
        <Command className="h-4 w-4 text-zinc-500" aria-hidden="true" />
        <input
          ref={commandInputRef}
          value={commandQuery}
          onChange={(event) => onCommandQueryChange(event.target.value)}
          placeholder={reviewSearchPlaceholder(locale)}
          className="h-full min-w-0 flex-1 bg-transparent text-sm text-zinc-100 outline-none placeholder:text-zinc-500"
          aria-label="Review search"
        />
        <Kbd>⌘K</Kbd>
      </div>
      <Badge tone={status.github === "connected" ? "green" : "amber"}>
        <Github className="mr-1 h-3 w-3" />
        GitHub {status.github}
      </Badge>
      <Badge tone={status.chatgpt === "connected" ? "green" : "amber"}>
        <Brain className="mr-1 h-3 w-3" />
        Codex {status.chatgpt}
      </Badge>
      <Badge tone="blue">{model}</Badge>
      <Badge tone="neutral">
        {languagePreferences.ui_locale}/{languagePreferences.review_locale}
      </Badge>
    </header>
  );
}

function AuthSetupScreen({
  locale,
  status,
  languagePreferences,
  oauthMessage,
  onConnectGithub,
  onConnectGithubDevice,
  onConnectChatGpt,
  onConnectChatGptDevice,
  onContinueWithoutAi,
  onSetUiLocale,
  onSetReviewLocale,
}: {
  locale: Locale;
  status: AppStatusView;
  languagePreferences: LanguagePreferences;
  oauthMessage: string | null;
  onConnectGithub: () => void;
  onConnectGithubDevice: () => void;
  onConnectChatGpt: () => void;
  onConnectChatGptDevice: () => void;
  onContinueWithoutAi: () => void;
  onSetUiLocale: (locale: Locale) => void;
  onSetReviewLocale: (locale: Locale) => void;
}) {
  const canContinueWithoutAi =
    status.github === "connected" && status.chatgpt !== "connected" && !status.strict_auth_mode;

  return (
    <div className="mx-auto flex min-h-screen max-w-5xl items-center px-8">
      <div className="grid w-full gap-6 lg:grid-cols-[1fr_420px]">
        <section>
          <Badge tone="blue">{t(locale, "app.name")} 0.1.0</Badge>
          <h1 className="mt-5 max-w-2xl text-4xl font-semibold tracking-normal">{t(locale, "auth.title")}</h1>
          <p className="mt-4 max-w-2xl text-sm leading-6 text-zinc-400">{t(locale, "auth.description")}</p>
          <div className="mt-8 grid gap-3">
            <ConnectionRow label={t(locale, "auth.github")} state={status.github} required />
            <ConnectionRow label={t(locale, "auth.chatgpt")} state={status.chatgpt} />
          </div>
          {oauthMessage && (
            <div className="mt-5 rounded-md border border-amber-800 bg-amber-950/60 p-3 text-sm text-amber-200">
              {oauthMessage}
            </div>
          )}
        </section>
        <Panel className="p-5">
          <div className="grid gap-4">
            <Button onClick={onConnectGithub} disabled={status.github === "connected"}>
              <Github className="h-4 w-4" />
              {t(locale, "auth.connectGithub")}
            </Button>
            <Button variant="ghost" onClick={onConnectGithubDevice} disabled={status.github === "connected"}>
              <Github className="h-4 w-4" />
              {t(locale, "auth.useGithubDeviceCode")}
            </Button>
            <Button variant="ghost" onClick={onConnectChatGpt} disabled={status.chatgpt === "connected"}>
              <Brain className="h-4 w-4" />
              {t(locale, "auth.connectChatGPT")}
            </Button>
            <Button variant="ghost" onClick={onConnectChatGptDevice} disabled={status.chatgpt === "connected"}>
              <Brain className="h-4 w-4" />
              {t(locale, "auth.useChatGptDeviceCode")}
            </Button>
            {canContinueWithoutAi && (
              <Button variant="ghost" onClick={onContinueWithoutAi}>
                <Bot className="h-4 w-4" />
                {t(locale, "auth.continueWithoutAi")}
              </Button>
            )}
            <LanguageSelect label={t(locale, "auth.uiLanguage")} value={languagePreferences.ui_locale} onChange={onSetUiLocale} />
            <LanguageSelect label={t(locale, "auth.reviewLanguage")} value={languagePreferences.review_locale} onChange={onSetReviewLocale} />
          </div>
        </Panel>
      </div>
    </div>
  );
}

function SettingsScreen({
  locale,
  status,
  codexBridgeStatus,
  languagePreferences,
  onSetUiLocale,
  onSetReviewLocale,
}: {
  locale: Locale;
  status: AppStatusView;
  codexBridgeStatus: CodexBridgeStatusView | null;
  languagePreferences: LanguagePreferences;
  onSetUiLocale: (locale: Locale) => void;
  onSetReviewLocale: (locale: Locale) => void;
}) {
  return (
    <div className="h-full overflow-auto p-5">
      <h1 className="mb-5 flex items-center gap-2 text-lg font-semibold">
        <Settings className="h-5 w-5" />
        {t(locale, "settings.title")}
      </h1>
      <div className="grid max-w-3xl gap-4 md:grid-cols-2">
        <Panel className="p-4">
          <h2 className="mb-3 text-sm font-semibold">{t(locale, "settings.connections")}</h2>
          <ConnectionRow label="GitHub" state={status.github} required />
          <div className="mt-2" />
          <ConnectionRow label="Codex" state={status.ai_connection?.status ?? status.chatgpt} />
          <div className="mt-3 grid gap-2 rounded-md border border-zinc-800 bg-zinc-900 p-3 text-xs text-zinc-400">
            <span>Auth mode: {status.ai_connection?.authMode ?? "not connected"}</span>
            <span>Plan: {status.ai_connection?.planType ?? "Plan unknown"}</span>
            <span>Account: {status.ai_connection?.account?.displayLabel ?? "No Codex account"}</span>
            <span>Model sync: {(status.ai_models?.length ?? 0) > 0 ? `${status.ai_models?.length} models` : "model_list_unavailable"}</span>
            <span>Rate limit: {status.ai_rate_limit?.status ?? "unknown"}</span>
            <span>Bridge: {codexBridgeStatus?.transport ?? "unavailable"}</span>
            <span>Codex CLI: {codexBridgeStatus?.version ?? codexBridgeStatus?.blockedReason ?? "not checked"}</span>
            <span>Blocked: {status.generation_blocked_reason ?? status.ai_connection?.blockedReason ?? "none"}</span>
          </div>
        </Panel>
        <Panel className="p-4">
          <h2 className="mb-3 flex items-center gap-2 text-sm font-semibold">
            <Languages className="h-4 w-4" />
            {t(locale, "settings.language")}
          </h2>
          <div className="grid gap-3">
            <LanguageSelect label={t(locale, "auth.uiLanguage")} value={languagePreferences.ui_locale} onChange={onSetUiLocale} />
            <LanguageSelect label={t(locale, "auth.reviewLanguage")} value={languagePreferences.review_locale} onChange={onSetReviewLocale} />
          </div>
        </Panel>
      </div>
    </div>
  );
}

function ConnectionRow({ label, state, required = false }: { label: string; state: string; required?: boolean }) {
  return (
    <div className="flex items-center justify-between rounded-md border border-zinc-800 bg-zinc-900 p-3">
      <span className="text-sm font-medium">{label}</span>
      <div className="flex items-center gap-2">
        {required && <Badge tone="amber">required</Badge>}
        <Badge tone={state === "connected" ? "green" : "amber"}>{state}</Badge>
      </div>
    </div>
  );
}

function LanguageSelect({ label, value, onChange }: { label: string; value: Locale; onChange: (locale: Locale) => void }) {
  return (
    <label className="grid gap-1 text-xs text-zinc-500">
      {label}
      <select value={value} onChange={(event) => onChange(event.target.value as Locale)} className="h-9 rounded-md border border-zinc-800 bg-zinc-900 px-2 text-sm text-zinc-100">
        <option value="en">English</option>
        <option value="ko">한국어</option>
      </select>
    </label>
  );
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

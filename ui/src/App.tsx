import {
  AlertTriangle,
  Bot,
  Brain,
  CheckCircle2,
  Command,
  FileCode2,
  Github,
  GitPullRequest,
  Languages,
  LockKeyhole,
  RefreshCw,
  Search,
  Send,
  Settings,
  ShieldCheck,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Badge, Button, Kbd, Panel } from "./components/ui";
import {
  collectPrContext,
  confirmSubmitReview,
  getAppStatus,
  getCodexBridgeStatus,
  listAiModels,
  listRepositories,
  loadReviewQueue,
  openExternalUrl,
  pollCodexChatGptLogin,
  pollGithubOAuth,
  prepareSubmitReview,
  refreshAiAccountStatus,
  refreshGithubAuthStatus,
  startAgentRun,
  startCodexChatGptLogin,
  startGithubOAuth,
  type SubmitPreflightView,
} from "./lib/ipc";
import { t } from "./lib/i18n";
import {
  commandDisabledReason,
  defaultLanguagePreferences,
  deriveSubmitSafetyState,
  groupQueueBySection,
  parseUnifiedPatch,
  prRiskBadges,
  pullRequestRef,
  reasoningEffortsForModel,
  reviewSearchPlaceholder,
  sampleStatus,
  selectableAiModels,
  selectedAiModel,
  shouldShowStartupGate,
  summarizeQueueItem,
  type AiModelView,
  type AppStatusView,
  type ChangedFileContext,
  type CodexBridgeStatusView,
  type DiffHunk,
  type InboxSection,
  type LanguagePreferences,
  type Locale,
  type PullRequestContextView,
  type PullRequestQueueItem,
  type ReasoningEffort,
  type Repository,
  type RiskBadge,
} from "./lib/view-models";
import {
  authPollTerminalMessage,
  shouldPollCodexLogin,
  shouldPollGithubOAuth,
} from "./lib/auth-flow";

const verdicts = ["COMMENT", "APPROVE", "REQUEST_CHANGES"] as const;
const appScreens = ["inbox", "settings"] as const;
const railPanels = ["summary", "ai", "draft", "safety", "activity"] as const;
const authPollMaxAttempts = 90;

type Verdict = (typeof verdicts)[number];
type Screen = "setup" | (typeof appScreens)[number];
type RailPanel = (typeof railPanels)[number];
type LoadState = "idle" | "loading" | "ready" | "error";

export default function App() {
  const commandInputRef = useRef<HTMLInputElement>(null);
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
  const [activeRailPanel, setActiveRailPanel] = useState<RailPanel>("summary");
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
    if (selectedPr && draft.trim()) refs.add(pullRequestRef(selectedPr));
    return refs;
  }, [draft, selectedPr]);

  const inboxSections = useMemo(
    () => groupQueueBySection(filteredQueue, { draftRefs, submittedRefs }),
    [draftRefs, filteredQueue, submittedRefs],
  );

  const safetyState = deriveSubmitSafetyState({
    hasTarget: Boolean(selectedPr && context),
    draftBody: draft,
    draftDirtySincePreflight,
    preflight,
    submitting,
    submittedReviewId,
    submitMessage,
  });

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
    setContextState("loading");
    setContextError(null);
    setPreflight(null);
    setDraftDirtySincePreflight(true);
    setSubmittedReviewId(null);
    setActiveRailPanel("summary");
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

  async function runAgent() {
    const disabled = commandDisabledReason(status, "generate_review_draft");
    if (disabled && disabled !== "generation_adapter_unavailable") {
      setAgentStatus("blocked");
      setAgentMessage(disabled);
      setActiveRailPanel("ai");
      return;
    }
    if (!selectedPr || !context) {
      setAgentStatus("blocked");
      setAgentMessage("Select a PR before running agents.");
      setActiveRailPanel("ai");
      return;
    }
    setAgentStatus("running");
    setAgentMessage(`Running ${model} with ${reasoningDepth} reasoning in ${languagePreferences.review_locale}`);
    setActiveRailPanel("ai");
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
    if (result.body.trim()) {
      setDraft(result.body);
      setDraftDirtySincePreflight(true);
      setPreflight(null);
      setActiveRailPanel("draft");
    }
  }

  async function prepareSubmit() {
    if (!context) return;
    setSubmitMessage(null);
    const next = await prepareSubmitReview({
      github_connected: status.github === "connected",
      write_scope_valid: true,
      sso_required: false,
      pr_open: context.pr.state === "open",
      pr_merged: context.pr.merged,
      expected_head_sha: context.pr.head_sha,
      current_head_sha: context.pr.head_sha,
      draft_body: draft,
      event: verdict,
      explicit_verdict_confirmed: explicitVerdict,
      private_diff_consent_required: agentStatus !== "blocked",
      private_diff_consent_accepted: privateConsent,
    });
    setPreflight(next);
    setDraftDirtySincePreflight(false);
  }

  async function confirmSubmit() {
    if (!selectedPr || !context || preflight?.status !== "ready" || !preflight.confirmation_id) return;
    setSubmitting(true);
    setSubmitMessage("Submitting review...");
    try {
      const submitted = await confirmSubmitReview({
        owner: selectedPr.owner,
        repo: selectedPr.repo,
        number: selectedPr.number,
        expected_head_sha: context.pr.head_sha,
        body: draft,
        event: verdict,
        explicit_verdict_confirmed: explicitVerdict,
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
    setDraftDirtySincePreflight(true);
    setPreflight(null);
    setSubmitMessage(null);
    setSubmittedReviewId(null);
  }

  function updateVerdict(next: Verdict) {
    setVerdict(next);
    setExplicitVerdict(next === "COMMENT");
    setDraftDirtySincePreflight(true);
    setPreflight(null);
    setSubmitMessage(null);
    setSubmittedReviewId(null);
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
      <div className="flex h-screen flex-col overflow-hidden">
        <TopBar
          locale={locale}
          status={status}
          model={model}
          languagePreferences={languagePreferences}
          commandInputRef={commandInputRef}
          commandQuery={commandQuery}
          onCommandQueryChange={setCommandQuery}
        />

        <div className="flex min-h-0 flex-1 overflow-hidden">
          <nav className="w-44 shrink-0 border-r border-zinc-800 bg-zinc-950 p-3">
            <div className="mb-4 px-2 text-sm font-semibold">{t(locale, "app.name")}</div>
            {appScreens.map((screen) => (
              <button
                key={screen}
                onClick={() => setActiveScreen(screen)}
                className={`mb-1 flex h-9 w-full items-center rounded-md px-2 text-left text-sm transition ${
                  activeScreen === screen ? "bg-zinc-100 text-zinc-950" : "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100"
                }`}
              >
                {screen === "inbox" ? t(locale, "nav.inbox") : t(locale, "nav.settings")}
              </button>
            ))}
          </nav>

          {activeScreen === "settings" ? (
            <section className="min-w-0 flex-1 overflow-hidden">
              <SettingsScreen
                locale={locale}
                status={status}
                codexBridgeStatus={codexBridgeStatus}
                languagePreferences={languagePreferences}
                onSetUiLocale={setUiLocale}
                onSetReviewLocale={setReviewLocale}
              />
            </section>
          ) : (
            <section className="grid min-w-0 flex-1 grid-cols-[320px_minmax(0,1fr)_360px] overflow-hidden max-[1023px]:grid-cols-[280px_minmax(0,1fr)]">
              <ReviewInboxRail
                locale={locale}
                repositories={repositories}
                selectedRepo={selectedRepo}
                filter={filter}
                queueState={queueState}
                queueError={queueError}
                sections={inboxSections}
                selectedPr={selectedPr}
                onSelectRepo={setSelectedRepo}
                onSetFilter={setFilter}
                onRefresh={() => refreshQueue(false)}
                onSelectPr={selectPullRequest}
              />
              <PrWorkspace
                locale={locale}
                selectedPr={selectedPr}
                context={context}
                contextState={contextState}
                contextError={contextError}
                selectedFile={selectedFile}
                viewedFiles={viewedFiles}
                onSelectFile={setSelectedFile}
                onMarkViewed={(path) => setViewedFiles((current) => new Set(current).add(path))}
                onRetryContext={() => selectedPr && selectPullRequest(selectedPr)}
              />
              <ReviewRail
                locale={locale}
                status={status}
                selectedRef={selectedRef}
                selectedPr={selectedPr}
                context={context}
                activeRailPanel={activeRailPanel}
                onSetActiveRailPanel={setActiveRailPanel}
                agentStatus={agentStatus}
                agentMessage={agentMessage}
                model={model}
                reasoningDepth={reasoningDepth}
                aiModels={status.ai_models ?? []}
                aiRateLimit={status.ai_rate_limit}
                aiConnection={status.ai_connection}
                reviewLocale={languagePreferences.review_locale}
                privateConsent={privateConsent}
                onSetModel={setModel}
                onSetReasoningDepth={setReasoningDepth}
                onSetPrivateConsent={(accepted) => {
                  setPrivateConsent(accepted);
                  setDraftDirtySincePreflight(true);
                  setPreflight(null);
                }}
                onRun={runAgent}
                verdict={verdict}
                draft={draft}
                explicitVerdict={explicitVerdict}
                onSetVerdict={updateVerdict}
                onSetExplicitVerdict={(confirmed) => {
                  setExplicitVerdict(confirmed);
                  setDraftDirtySincePreflight(true);
                  setPreflight(null);
                }}
                onSetDraft={updateDraft}
                safetyState={safetyState}
                preflight={preflight}
                submitMessage={submitMessage}
                onPrepare={prepareSubmit}
                onConfirm={confirmSubmit}
              />
            </section>
          )}
        </div>
      </div>
      <div className="sr-only">
        <Search />
      </div>
    </main>
  );
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

function ReviewInboxRail({
  locale,
  repositories,
  selectedRepo,
  filter,
  queueState,
  queueError,
  sections,
  selectedPr,
  onSelectRepo,
  onSetFilter,
  onRefresh,
  onSelectPr,
}: {
  locale: Locale;
  repositories: Repository[];
  selectedRepo: string | null;
  filter: "all" | "requested" | "assigned" | "ci_failed";
  queueState: LoadState;
  queueError: string | null;
  sections: InboxSection[];
  selectedPr: PullRequestQueueItem | null;
  onSelectRepo: (repo: string | null) => void;
  onSetFilter: (filter: "all" | "requested" | "assigned" | "ci_failed") => void;
  onRefresh: () => void;
  onSelectPr: (pr: PullRequestQueueItem) => void;
}) {
  const totalItems = sections.reduce((count, section) => count + section.items.length, 0);
  return (
    <aside className="min-h-0 border-r border-zinc-800 bg-zinc-950">
      <div className="border-b border-zinc-800 p-3">
        <div className="mb-3 flex items-center justify-between">
          <h1 className="text-sm font-semibold">{t(locale, "inbox.title")}</h1>
          <Button variant="ghost" className="h-8 px-2" onClick={onRefresh} disabled={queueState === "loading"}>
            <RefreshCw className={`h-4 w-4 ${queueState === "loading" ? "animate-spin" : ""}`} />
            {t(locale, "inbox.refresh")}
          </Button>
        </div>
        <select
          value={selectedRepo ?? "all"}
          onChange={(event) => onSelectRepo(event.target.value === "all" ? null : event.target.value)}
          className="h-9 w-full rounded-md border border-zinc-800 bg-zinc-900 px-2 text-sm text-zinc-100"
          aria-label="Repository picker"
        >
          <option value="all">{t(locale, "inbox.allRepos")}</option>
          {repositories.map((repo) => (
            <option key={repo.full_name} value={repo.full_name}>
              {repo.full_name}
            </option>
          ))}
        </select>
        <div className="mt-3 grid grid-cols-2 gap-2">
          {[
            ["all", t(locale, "inbox.filter.all")],
            ["requested", t(locale, "inbox.filter.requested")],
            ["assigned", t(locale, "inbox.filter.assigned")],
            ["ci_failed", t(locale, "inbox.filter.ci_failed")],
          ].map(([value, label]) => (
            <Button key={value} variant={filter === value ? "default" : "ghost"} className="h-8" onClick={() => onSetFilter(value as typeof filter)}>
              {label}
            </Button>
          ))}
        </div>
      </div>
      {queueState === "loading" && <StatusState title="Loading review inbox" />}
      {queueState === "error" && <StatusState title="Could not load review inbox" message={queueError ?? "review_queue_failed"} actionLabel="Retry" onAction={onRefresh} />}
      {queueState !== "loading" && queueState !== "error" && totalItems === 0 && <StatusState title={t(locale, "inbox.empty")} />}
      {queueState !== "loading" && queueState !== "error" && totalItems > 0 && (
        <div className="h-full overflow-auto pb-10">
          {sections.map((section) => (
            <section key={section.id} className="border-b border-zinc-900">
              <div className="sticky top-0 z-10 flex items-center justify-between bg-zinc-950/95 px-3 py-2 backdrop-blur">
                <div>
                  <h2 className="text-xs font-semibold text-zinc-200">{sectionLabel(locale, section.id)}</h2>
                  <p className="text-[11px] text-zinc-500">{section.items.length} PRs</p>
                </div>
              </div>
              {section.items.length === 0 ? (
                <div className="px-3 pb-3 text-xs text-zinc-600">No items</div>
              ) : (
                section.items.map((item) => (
                  <PullRequestCard
                    key={`${section.id}-${pullRequestRef(item)}`}
                    item={item}
                    active={selectedPr ? pullRequestRef(selectedPr) === pullRequestRef(item) : false}
                    onSelect={() => onSelectPr(item)}
                  />
                ))
              )}
            </section>
          ))}
        </div>
      )}
    </aside>
  );
}

function PullRequestCard({ item, active, onSelect }: { item: PullRequestQueueItem; active: boolean; onSelect: () => void }) {
  const summary = summarizeQueueItem(item);
  const risks = prRiskBadges(item);
  return (
    <button
      onClick={onSelect}
      className={`grid w-full gap-2 border-t border-zinc-900 px-3 py-3 text-left transition hover:bg-zinc-900 ${
        active ? "bg-zinc-900" : ""
      }`}
    >
      <div className="flex min-w-0 items-center justify-between gap-2">
        <span className="truncate text-xs text-zinc-400">{summary.ref}</span>
        <Badge tone={item.ci_status === "fail" ? "red" : item.ci_status === "pass" ? "green" : "amber"}>{summary.ci}</Badge>
      </div>
      <div className="line-clamp-2 text-sm font-medium text-zinc-100">{item.title}</div>
      <div className="flex items-center justify-between text-xs text-zinc-500">
        <span>{summary.reason}</span>
        <span>{summary.files}</span>
      </div>
      {risks.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {risks.map((badge) => (
            <Badge key={badge.id} tone={badge.tone} className="h-5 text-[10px]">
              {badge.label}
            </Badge>
          ))}
        </div>
      )}
    </button>
  );
}

function PrWorkspace({
  locale,
  selectedPr,
  context,
  contextState,
  contextError,
  selectedFile,
  viewedFiles,
  onSelectFile,
  onMarkViewed,
  onRetryContext,
}: {
  locale: Locale;
  selectedPr: PullRequestQueueItem | null;
  context: PullRequestContextView | null;
  contextState: LoadState;
  contextError: string | null;
  selectedFile: ChangedFileContext | null;
  viewedFiles: Set<string>;
  onSelectFile: (file: ChangedFileContext) => void;
  onMarkViewed: (path: string) => void;
  onRetryContext: () => void;
}) {
  if (!selectedPr) return <EmptyWorkspace label={t(locale, "shell.noTarget")} />;
  if (contextState === "loading") return <StatusState title="Loading PR context" message={pullRequestRef(selectedPr)} />;
  if (contextState === "error") {
    return <StatusState title="Could not load PR context" message={contextError ?? "pr_context_failed"} actionLabel="Retry" onAction={onRetryContext} />;
  }
  if (!context) return <EmptyWorkspace label="Select a PR to load changed files and diff." />;

  return (
    <div className="grid min-w-0 grid-rows-[auto_minmax(0,1fr)] overflow-hidden bg-zinc-950">
      <PrOverview locale={locale} context={context} />
      <div className="grid min-h-0 grid-cols-[260px_minmax(0,1fr)] overflow-hidden max-[1023px]:grid-cols-[220px_minmax(0,1fr)]">
        <div className="min-h-0 border-r border-zinc-800">
          <div className="flex h-11 items-center gap-2 border-b border-zinc-800 px-3 text-sm font-medium">
            <FileCode2 className="h-4 w-4 text-zinc-500" />
            {t(locale, "workspace.changedFiles")}
          </div>
          <div className="h-full overflow-auto pb-12">
            {context.files.map((file) => {
              const active = selectedFile?.path === file.path;
              const viewed = viewedFiles.has(file.path);
              return (
                <button
                  key={file.path}
                  onClick={() => onSelectFile(file)}
                  className={`grid w-full gap-1 border-b border-zinc-900 px-3 py-2 text-left text-sm hover:bg-zinc-900 ${
                    active ? "bg-zinc-900 text-white" : "text-zinc-300"
                  }`}
                >
                  <div className="flex min-w-0 items-center justify-between gap-2">
                    <span className={`truncate ${viewed ? "text-zinc-500" : ""}`}>{file.path}</span>
                    <Badge tone={file.patch ? "blue" : "neutral"} className="h-5 text-[10px]">
                      {file.status}
                    </Badge>
                  </div>
                  <div className="flex items-center justify-between text-[11px] text-zinc-500">
                    <span>+{file.additions} -{file.deletions}</span>
                    {viewed && <span>{t(locale, "workspace.fileViewed")}</span>}
                  </div>
                </button>
              );
            })}
          </div>
        </div>
        <DiffViewer locale={locale} file={selectedFile} viewed={selectedFile ? viewedFiles.has(selectedFile.path) : false} onMarkViewed={onMarkViewed} />
      </div>
    </div>
  );
}

function PrOverview({ locale, context }: { locale: Locale; context: PullRequestContextView }) {
  return (
    <header className="border-b border-zinc-800 bg-zinc-950 px-4 py-3">
      <div className="flex min-w-0 items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="mb-1 flex flex-wrap items-center gap-2 text-xs text-zinc-500">
            <GitPullRequest className="h-4 w-4" />
            {context.pr.owner}/{context.pr.repo}#{context.pr.number}
            <span>by {context.pr.author}</span>
            <span>head {context.pr.head_sha.slice(0, 8)}</span>
          </div>
          <h2 className="truncate text-lg font-semibold">{context.pr.title}</h2>
        </div>
        <div className="flex shrink-0 flex-wrap justify-end gap-2">
          <Badge tone="green">+{context.pr.additions}</Badge>
          <Badge tone="red">-{context.pr.deletions}</Badge>
          <Badge tone={context.ci.state === "failure" ? "red" : context.ci.state === "success" ? "green" : "amber"}>CI {context.ci.state}</Badge>
          <Badge tone={context.freshness === "fresh" ? "green" : "amber"}>{t(locale, `stale.${context.freshness}`)}</Badge>
          <Badge tone="neutral">
            {t(locale, "workspace.aiIncluded")}: {context.ai_input.included_file_count}/{context.files.length}
          </Badge>
        </div>
      </div>
    </header>
  );
}

function DiffViewer({
  locale,
  file,
  viewed,
  onMarkViewed,
}: {
  locale: Locale;
  file: ChangedFileContext | null;
  viewed: boolean;
  onMarkViewed: (path: string) => void;
}) {
  if (!file) return <EmptyWorkspace label="Select a changed file." />;
  const hunks = parseUnifiedPatch(file.patch);
  return (
    <div className="min-w-0 overflow-hidden">
      <div className="sticky top-0 z-10 flex min-h-12 items-center justify-between gap-3 border-b border-zinc-800 bg-zinc-950 px-4">
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">{file.path}</div>
          <div className="text-xs text-zinc-500">
            {file.status} · {file.patch_coverage} · {file.changes} changes
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {hunks.map((hunk, index) => (
            <Button
              key={hunk.header}
              variant="ghost"
              className="h-7 px-2 text-xs"
              onClick={() => document.getElementById(`hunk-${index}`)?.scrollIntoView({ block: "start" })}
            >
              H{index + 1}
            </Button>
          ))}
          <Button variant={viewed ? "default" : "ghost"} className="h-7 px-2 text-xs" onClick={() => onMarkViewed(file.path)}>
            {viewed ? t(locale, "workspace.fileViewed") : t(locale, "workspace.markViewed")}
          </Button>
        </div>
      </div>
      {!file.patch || hunks.length === 0 ? (
        <StatusState title={t(locale, "workspace.noPatch")} message={`${file.status} · ${file.patch_coverage}`} />
      ) : (
        <div className="h-full overflow-auto pb-20 font-mono text-[12px] leading-5">
          {hunks.map((hunk: DiffHunk, index) => (
            <section id={`hunk-${index}`} key={`${hunk.header}-${index}`} className="border-b border-zinc-900">
              <div className="bg-zinc-900 px-4 py-2 text-xs text-sky-300">{hunk.header}</div>
              {hunk.lines.map((line, lineIndex) => (
                <div
                  key={`${index}-${lineIndex}`}
                  className={`grid grid-cols-[56px_56px_minmax(0,1fr)] border-b border-zinc-950/80 ${
                    line.type === "add"
                      ? "bg-emerald-950/40 text-emerald-100"
                      : line.type === "delete"
                        ? "bg-red-950/40 text-red-100"
                        : "text-zinc-300"
                  }`}
                >
                  <span className="select-none border-r border-zinc-800 px-2 text-right text-zinc-600">{line.oldLine ?? ""}</span>
                  <span className="select-none border-r border-zinc-800 px-2 text-right text-zinc-600">{line.newLine ?? ""}</span>
                  <span className="whitespace-pre px-3">{line.content}</span>
                </div>
              ))}
            </section>
          ))}
        </div>
      )}
    </div>
  );
}

function ReviewRail({
  locale,
  status,
  selectedRef,
  selectedPr,
  context,
  activeRailPanel,
  onSetActiveRailPanel,
  agentStatus,
  agentMessage,
  model,
  reasoningDepth,
  aiModels,
  aiRateLimit,
  aiConnection,
  reviewLocale,
  privateConsent,
  onSetModel,
  onSetReasoningDepth,
  onSetPrivateConsent,
  onRun,
  verdict,
  draft,
  explicitVerdict,
  onSetVerdict,
  onSetExplicitVerdict,
  onSetDraft,
  safetyState,
  preflight,
  submitMessage,
  onPrepare,
  onConfirm,
}: {
  locale: Locale;
  status: AppStatusView;
  selectedRef: string;
  selectedPr: PullRequestQueueItem | null;
  context: PullRequestContextView | null;
  activeRailPanel: RailPanel;
  onSetActiveRailPanel: (panel: RailPanel) => void;
  agentStatus: string;
  agentMessage: string;
  model: string;
  reasoningDepth: ReasoningEffort;
  aiModels: AiModelView[];
  aiRateLimit: AppStatusView["ai_rate_limit"];
  aiConnection: AppStatusView["ai_connection"];
  reviewLocale: Locale;
  privateConsent: boolean;
  onSetModel: (model: string) => void;
  onSetReasoningDepth: (depth: ReasoningEffort) => void;
  onSetPrivateConsent: (accepted: boolean) => void;
  onRun: () => void;
  verdict: Verdict;
  draft: string;
  explicitVerdict: boolean;
  onSetVerdict: (verdict: Verdict) => void;
  onSetExplicitVerdict: (confirmed: boolean) => void;
  onSetDraft: (draft: string) => void;
  safetyState: ReturnType<typeof deriveSubmitSafetyState>;
  preflight: SubmitPreflightView | null;
  submitMessage: string | null;
  onPrepare: () => void;
  onConfirm: () => void;
}) {
  return (
    <aside className="min-h-0 border-l border-zinc-800 bg-zinc-950 max-[1023px]:hidden">
      <div className="border-b border-zinc-800 p-3">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-semibold">{t(locale, "shell.reviewRail")}</h2>
          <Badge tone={selectedPr ? "blue" : "neutral"}>{selectedRef}</Badge>
        </div>
        <div className="grid grid-cols-5 gap-1">
          {railPanels.map((panel) => (
            <button
              key={panel}
              onClick={() => onSetActiveRailPanel(panel)}
              className={`h-8 rounded-md text-xs ${
                activeRailPanel === panel ? "bg-zinc-100 text-zinc-950" : "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100"
              }`}
            >
              {railLabel(locale, panel)}
            </button>
          ))}
        </div>
      </div>
      <div className="h-full overflow-auto p-3 pb-20">
        {activeRailPanel === "summary" && <SummaryPanel locale={locale} context={context} />}
        {activeRailPanel === "ai" && (
          <AiRailPanel
            locale={locale}
            status={status}
            selectedRef={selectedRef}
            context={context}
            agentStatus={agentStatus}
            agentMessage={agentMessage}
            model={model}
            reasoningDepth={reasoningDepth}
            aiModels={aiModels}
            aiRateLimit={aiRateLimit}
            aiConnection={aiConnection}
            reviewLocale={reviewLocale}
            privateConsent={privateConsent}
            onSetModel={onSetModel}
            onSetReasoningDepth={onSetReasoningDepth}
            onSetPrivateConsent={onSetPrivateConsent}
            onRun={onRun}
          />
        )}
        {activeRailPanel === "draft" && (
          <DraftRailPanel
            locale={locale}
            verdict={verdict}
            draft={draft}
            explicitVerdict={explicitVerdict}
            onSetVerdict={onSetVerdict}
            onSetExplicitVerdict={onSetExplicitVerdict}
            onSetDraft={onSetDraft}
          />
        )}
        {activeRailPanel === "safety" && (
          <SafetyRailPanel
            locale={locale}
            safetyState={safetyState}
            preflight={preflight}
            submitMessage={submitMessage}
            onPrepare={onPrepare}
            onConfirm={onConfirm}
          />
        )}
        {activeRailPanel === "activity" && <ActivityPanel context={context} />}
      </div>
    </aside>
  );
}

function SummaryPanel({ locale, context }: { locale: Locale; context: PullRequestContextView | null }) {
  if (!context) return <StatusState title={t(locale, "shell.noTarget")} />;
  return (
    <div className="grid gap-3">
      <RailMetric label="Freshness" value={t(locale, `stale.${context.freshness}`)} />
      <RailMetric label="Patch coverage" value={context.patch_coverage} />
      <RailMetric label="AI input" value={`${context.ai_input.included_file_count}/${context.files.length} files`} />
      <RailMetric label="CI" value={`${context.ci.state} · ${context.ci.source}`} />
      {context.warnings.length > 0 && (
        <div className="rounded-md border border-amber-800 bg-amber-950/50 p-3 text-xs text-amber-200">
          {context.warnings.join(", ")}
        </div>
      )}
    </div>
  );
}

function AiRailPanel({
  locale,
  status,
  selectedRef,
  context,
  agentStatus,
  agentMessage,
  model,
  reasoningDepth,
  aiModels,
  aiRateLimit,
  aiConnection,
  reviewLocale,
  privateConsent,
  onSetModel,
  onSetReasoningDepth,
  onSetPrivateConsent,
  onRun,
}: {
  locale: Locale;
  status: AppStatusView;
  selectedRef: string;
  context: PullRequestContextView | null;
  agentStatus: string;
  agentMessage: string;
  model: string;
  reasoningDepth: ReasoningEffort;
  aiModels: AiModelView[];
  aiRateLimit: AppStatusView["ai_rate_limit"];
  aiConnection: AppStatusView["ai_connection"];
  reviewLocale: Locale;
  privateConsent: boolean;
  onSetModel: (model: string) => void;
  onSetReasoningDepth: (depth: ReasoningEffort) => void;
  onSetPrivateConsent: (accepted: boolean) => void;
  onRun: () => void;
}) {
  const disabled = commandDisabledReason(status, "generate_review_draft");
  const hardDisabled = Boolean(disabled && disabled !== "generation_adapter_unavailable");
  const models = selectableAiModels(aiModels);
  const selectedModel = selectedAiModel(aiModels, model);
  const efforts = reasoningEffortsForModel(aiModels, model);
  return (
    <div className="grid gap-3">
      <RailMetric label="Target" value={selectedRef} />
      <RailMetric label="Scope" value={context ? `${context.ai_input.included_file_count}/${context.files.length} files` : "No PR selected"} />
      <label className="grid gap-1 text-xs text-zinc-500">
        {t(locale, "agent.model")}
        <select value={model} onChange={(event) => onSetModel(event.target.value)} className="h-9 rounded-md border border-zinc-800 bg-zinc-900 px-2 text-sm text-zinc-100">
          {models.length === 0 && <option value={model}>No Codex models available</option>}
          {models.map((item) => (
            <option key={item.id} value={item.id} disabled={!item.available}>
              {item.displayName}{item.available ? "" : ` · ${item.unavailableReason ?? "unavailable"}`}
            </option>
          ))}
        </select>
      </label>
      <label className="grid gap-1 text-xs text-zinc-500">
        {t(locale, "agent.reasoning")}
        <select value={reasoningDepth} onChange={(event) => onSetReasoningDepth(event.target.value as ReasoningEffort)} className="h-9 rounded-md border border-zinc-800 bg-zinc-900 px-2 text-sm text-zinc-100">
          {(efforts.length > 0 ? efforts : [reasoningDepth]).map((effort) => (
            <option key={effort} value={effort}>
              {effort}
            </option>
          ))}
        </select>
      </label>
      <div className="grid gap-2 rounded-md border border-zinc-800 bg-zinc-900 p-3 text-xs text-zinc-400">
        <span>Auth {aiConnection?.status ?? status.chatgpt}</span>
        <span>Rate {aiRateLimit?.status ?? "unknown"}</span>
        <span>Model {selectedModel?.available ? "available" : selectedModel?.unavailableReason ?? "unavailable"}</span>
        <span>Review language {reviewLocale}</span>
      </div>
      <label className="flex items-center gap-2 rounded-md border border-zinc-800 bg-zinc-900 p-3 text-sm text-zinc-300">
        <input type="checkbox" checked={privateConsent} onChange={(event) => onSetPrivateConsent(event.target.checked)} />
        {t(locale, "agent.privateConsent")}
      </label>
      <Button onClick={onRun} disabled={hardDisabled}>
        <Bot className="h-4 w-4" />
        {t(locale, "agent.run")}
      </Button>
      <div className="rounded-md border border-zinc-800 bg-zinc-900 p-3 text-sm text-zinc-300">
        <div className="mb-1 flex items-center justify-between">
          <span>AI findings lane</span>
          <Badge tone={agentStatus === "blocked" ? "amber" : agentStatus === "failed" ? "red" : "blue"}>{agentStatus}</Badge>
        </div>
        <p className="text-xs text-zinc-500">{disabled ?? agentMessage}</p>
      </div>
    </div>
  );
}

function DraftRailPanel({
  locale,
  verdict,
  draft,
  explicitVerdict,
  onSetVerdict,
  onSetExplicitVerdict,
  onSetDraft,
}: {
  locale: Locale;
  verdict: Verdict;
  draft: string;
  explicitVerdict: boolean;
  onSetVerdict: (verdict: Verdict) => void;
  onSetExplicitVerdict: (confirmed: boolean) => void;
  onSetDraft: (draft: string) => void;
}) {
  return (
    <div className="grid gap-3">
      <div className="flex flex-wrap gap-2">
        {verdicts.map((value) => (
          <Button key={value} variant={verdict === value ? "default" : "ghost"} className="h-8 px-2 text-xs" onClick={() => onSetVerdict(value)}>
            {value === "REQUEST_CHANGES" ? "Changes" : value}
          </Button>
        ))}
      </div>
      {verdict !== "COMMENT" && (
        <label className="flex items-center gap-2 text-sm text-zinc-300">
          <input type="checkbox" checked={explicitVerdict} onChange={(event) => onSetExplicitVerdict(event.target.checked)} />
          {t(locale, "draft.confirm")} {verdict}
        </label>
      )}
      <textarea
        value={draft}
        onChange={(event) => onSetDraft(event.target.value)}
        placeholder={t(locale, "draft.empty")}
        className="h-[48vh] min-h-64 w-full resize-none rounded-md border border-zinc-800 bg-zinc-900 p-3 text-sm text-zinc-100 outline-none focus:border-sky-500"
        aria-label="Review draft body"
      />
    </div>
  );
}

function SafetyRailPanel({
  locale,
  safetyState,
  preflight,
  submitMessage,
  onPrepare,
  onConfirm,
}: {
  locale: Locale;
  safetyState: ReturnType<typeof deriveSubmitSafetyState>;
  preflight: SubmitPreflightView | null;
  submitMessage: string | null;
  onPrepare: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="grid gap-3">
      <div className="rounded-md border border-zinc-800 bg-zinc-900 p-3">
        <div className="mb-2 flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm font-medium">
            <ShieldCheck className="h-4 w-4 text-emerald-400" />
            {t(locale, "safety.title")}
          </div>
          <Badge tone={safetyTone(safetyState.status)}>{safetyState.status}</Badge>
        </div>
        <p className="text-xs text-zinc-500">{submitMessage ?? safetyState.label}</p>
      </div>
      {preflight?.blocked_reasons.map((reason) => (
        <Badge key={reason} tone="red">
          <AlertTriangle className="mr-1 h-3 w-3" />
          {t(locale, `reason.${reason}`)}
        </Badge>
      ))}
      {safetyState.status === "ready" && (
        <Badge tone="green">
          <CheckCircle2 className="mr-1 h-3 w-3" />
          {t(locale, "safety.ready")}
        </Badge>
      )}
      <Button variant="ghost" onClick={onPrepare} disabled={!safetyState.canPrepare}>
        <LockKeyhole className="h-4 w-4" />
        {t(locale, "safety.prepare")}
      </Button>
      <Button disabled={!safetyState.canConfirm} onClick={onConfirm}>
        <Send className="h-4 w-4" />
        {t(locale, "safety.confirm")}
      </Button>
    </div>
  );
}

function ActivityPanel({ context }: { context: PullRequestContextView | null }) {
  if (!context) return <StatusState title="No activity target" />;
  return (
    <div className="grid gap-3 text-sm">
      <RailMetric label="Issue comments" value={String(context.conversation.issue_comment_count)} />
      <RailMetric label="Review comments" value={String(context.conversation.review_comment_count)} />
      <RailMetric label="Reviews" value={String(context.conversation.review_count)} />
      {context.conversation.summaries.map((summary, index) => (
        <div key={`${summary.kind}-${index}`} className="rounded-md border border-zinc-800 bg-zinc-900 p-3 text-xs text-zinc-400">
          <div className="mb-1 text-zinc-200">{summary.kind} · {summary.author}</div>
          <div>{summary.body_summary}</div>
        </div>
      ))}
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

function StatusState({ title, message, actionLabel, onAction }: { title: string; message?: string | null; actionLabel?: string; onAction?: () => void }) {
  return (
    <div className="flex h-full min-h-48 items-center justify-center p-6 text-center text-zinc-500">
      <div className="max-w-sm">
        <FileCode2 className="mx-auto mb-3 h-8 w-8" />
        <div className="text-sm font-medium text-zinc-300">{title}</div>
        {message && <div className="mt-1 text-xs">{message}</div>}
        {onAction && (
          <Button variant="ghost" className="mt-4" onClick={onAction}>
            {actionLabel ?? "Retry"}
          </Button>
        )}
      </div>
    </div>
  );
}

function EmptyWorkspace({ label }: { label: string }) {
  return <StatusState title={label} />;
}

function RailMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-zinc-800 bg-zinc-900 p-3">
      <div className="text-[11px] uppercase tracking-wide text-zinc-500">{label}</div>
      <div className="mt-1 text-sm text-zinc-100">{value}</div>
    </div>
  );
}

function sectionLabel(locale: Locale, section: InboxSection["id"]): string {
  return t(locale, `inbox.section.${section}`);
}

function railLabel(locale: Locale, panel: RailPanel): string {
  return t(locale, `rail.${panel}`);
}

function safetyTone(status: ReturnType<typeof deriveSubmitSafetyState>["status"]): RiskBadge["tone"] {
  if (status === "ready" || status === "submitted") return "green";
  if (status === "blocked" || status === "failed") return "red";
  if (status === "stale" || status === "not_ready") return "amber";
  return "blue";
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

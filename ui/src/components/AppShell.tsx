import type { ReactNode } from "react";
import { AssignedPrQueue } from "./AssignedPrQueue";
import { DraftPublishPanel } from "./DraftPublishPanel";
import { PrWorkspace } from "./PrWorkspace";
import type { SubmitPreflightView } from "../lib/ipc";
import type { deriveSubmitSafetyState, ChangedFileContext, InboxSection, Locale, PullRequestContextView, PullRequestQueueItem, Repository } from "../lib/view-models";
import type { AnalysisRunMode, AnalysisRunView, ReviewDraftView, ReviewEvent, ReviewPublishPayloadView } from "../lib/workspace-view-models";

type LoadState = "idle" | "loading" | "ready" | "error";
type QueueFilter = "all" | "requested" | "assigned" | "ci_failed";

export function AppShell({
  topBar,
  activeScreen,
  navItems,
  onSetActiveScreen,
  settingsContent,
  locale,
  queueProps,
  workspaceProps,
  draftPublishProps,
}: {
  topBar: ReactNode;
  activeScreen: "inbox" | "settings";
  navItems: Array<{ id: "inbox" | "settings"; label: string }>;
  onSetActiveScreen: (screen: "inbox" | "settings") => void;
  settingsContent: ReactNode;
  locale: Locale;
  queueProps: {
    repositories: Repository[];
    selectedRepo: string | null;
    filter: QueueFilter;
    queueState: LoadState;
    queueError: string | null;
    sections: InboxSection[];
    selectedPr: PullRequestQueueItem | null;
    onSelectRepo: (repo: string | null) => void;
    onSetFilter: (filter: QueueFilter) => void;
    onRefresh: () => void;
    onSelectPr: (pr: PullRequestQueueItem) => void;
  };
  workspaceProps: {
    selectedPr: PullRequestQueueItem | null;
    context: PullRequestContextView | null;
    contextState: LoadState;
    contextError: string | null;
    selectedFile: ChangedFileContext | null;
    viewedFiles: Set<string>;
    onSelectFile: (file: ChangedFileContext) => void;
    onMarkViewed: (path: string) => void;
    onRetryContext: () => void;
  };
  draftPublishProps: {
    selectedRef: string;
    draft: ReviewDraftView | null;
    runs: AnalysisRunView[];
    explicitVerdict: boolean;
    safetyState: ReturnType<typeof deriveSubmitSafetyState>;
    preflight: SubmitPreflightView | null;
    submitMessage: string | null;
    publishPayload: ReviewPublishPayloadView | null;
    submitting: boolean;
    runDisabled?: boolean;
    onRunMode: (mode: AnalysisRunMode) => void;
    onRunCustom: () => void;
    onReplaceDraftFromRun: (run: AnalysisRunView) => void;
    onSetVerdict: (verdict: ReviewEvent) => void;
    onSetExplicitVerdict: (confirmed: boolean) => void;
    onSetDraftBody: (body: string) => void;
    onToggleInlineSelected: (id: string, selected: boolean) => void;
    onDismissInline: (id: string) => void;
    onPrepare: () => void;
    onConfirm: () => void;
  };
}) {
  return (
    <div className="flex h-screen flex-col overflow-hidden">
      {topBar}
      <div className="flex min-h-0 flex-1 overflow-hidden">
        <nav className="w-44 shrink-0 border-r border-zinc-800 bg-zinc-950 p-3">
          <div className="mb-4 px-2 text-sm font-semibold">ReviewDesk</div>
          {navItems.map((screen) => (
            <button key={screen.id} onClick={() => onSetActiveScreen(screen.id)} className={`mb-1 flex h-9 w-full items-center rounded-md px-2 text-left text-sm transition ${activeScreen === screen.id ? "bg-zinc-100 text-zinc-950" : "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100"}`}>
              {screen.label}
            </button>
          ))}
        </nav>

        {activeScreen === "settings" ? (
          <section className="min-w-0 flex-1 overflow-hidden">{settingsContent}</section>
        ) : (
          <section className="reviewdesk-workspace-grid min-w-0 flex-1 overflow-hidden">
            <AssignedPrQueue locale={locale} {...queueProps} />
            <PrWorkspace locale={locale} {...workspaceProps} />
            <DraftPublishPanel locale={locale} {...draftPublishProps} />
          </section>
        )}
      </div>
    </div>
  );
}

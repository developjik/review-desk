import { useEffect, useState } from "react";
import { Badge } from "./ui";
import { DraftBodyTab } from "./DraftBodyTab";
import { InlineCommentsTab } from "./InlineCommentsTab";
import { PublishConfirmation } from "./PublishConfirmation";
import { RunHistory } from "./RunHistory";
import { RunShelf } from "./RunShelf";
import { SafetyTab } from "./SafetyTab";
import type { SubmitPreflightView } from "../lib/ipc";
import type { deriveSubmitSafetyState, Locale } from "../lib/view-models";
import type { AnalysisRunMode, AnalysisRunView, InlineCommentDraftView, ReviewDraftView, ReviewEvent, ReviewPublishPayloadView } from "../lib/workspace-view-models";

type PanelTab = "draft" | "inline" | "safety" | "runs";

export function DraftPublishPanel({
  locale,
  selectedRef,
  draft,
  runs,
  explicitVerdict,
  safetyState,
  preflight,
  submitMessage,
  publishPayload,
  submitting,
  privateConsent,
  runDisabled,
  onRunMode,
  onRunCustom,
  onReplaceDraftFromRun,
  onSetVerdict,
  onSetExplicitVerdict,
  onSetDraftBody,
  onToggleInlineSelected,
  onDismissInline,
  onSetPrivateConsent,
  onPrepare,
  onConfirm,
}: {
  locale: Locale;
  selectedRef: string;
  draft: ReviewDraftView | null;
  runs: AnalysisRunView[];
  explicitVerdict: boolean;
  safetyState: ReturnType<typeof deriveSubmitSafetyState>;
  preflight: SubmitPreflightView | null;
  submitMessage: string | null;
  publishPayload: ReviewPublishPayloadView | null;
  submitting: boolean;
  privateConsent: boolean;
  runDisabled?: boolean;
  onRunMode: (mode: AnalysisRunMode) => void;
  onRunCustom: () => void;
  onReplaceDraftFromRun: (run: AnalysisRunView) => void;
  onSetVerdict: (verdict: ReviewEvent) => void;
  onSetExplicitVerdict: (confirmed: boolean) => void;
  onSetDraftBody: (body: string) => void;
  onToggleInlineSelected: (id: string, selected: boolean) => void;
  onDismissInline: (id: string) => void;
  onSetPrivateConsent: (accepted: boolean) => void;
  onPrepare: () => void;
  onConfirm: () => void;
}) {
  const [activeTab, setActiveTab] = useState<PanelTab>("draft");
  const [confirmOpen, setConfirmOpen] = useState(false);
  const comments: InlineCommentDraftView[] = draft?.inline_comments ?? [];

  useEffect(() => {
    setConfirmOpen(false);
  }, [selectedRef]);

  return (
    <aside className="reviewdesk-draft-panel min-h-0 border-l border-zinc-800 bg-zinc-950">
      <div className="border-b border-zinc-800 p-3">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-semibold">Review rail</h2>
          <Badge tone={draft ? "blue" : "neutral"}>{selectedRef}</Badge>
        </div>
        <div className="mb-3 text-xs text-zinc-500">Draft & Publish</div>
        <RunShelf
          disabled={runDisabled}
          privateConsent={privateConsent}
          onPrivateConsentChange={onSetPrivateConsent}
          onRunMode={onRunMode}
          onRunCustom={onRunCustom}
        />
        <div className="mt-3 grid grid-cols-4 gap-1">
          {(["draft", "inline", "safety", "runs"] as PanelTab[]).map((tab) => (
            <button key={tab} onClick={() => setActiveTab(tab)} className={`h-8 rounded-md text-xs ${activeTab === tab ? "bg-zinc-100 text-zinc-950" : "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100"}`}>
              {tab === "inline" ? "Inline" : tab === "runs" ? "Runs" : tab[0].toUpperCase() + tab.slice(1)}
            </button>
          ))}
        </div>
      </div>
      <div className="h-full overflow-auto p-3 pb-20">
        <div className="mb-4">
          <RunHistory runs={runs} onReplaceDraftFromRun={onReplaceDraftFromRun} />
        </div>
        {activeTab === "draft" && (
          <DraftBodyTab
            locale={locale}
            verdict={draft?.verdict ?? "COMMENT"}
            body={draft?.body ?? ""}
            explicitVerdict={explicitVerdict}
            onSetVerdict={onSetVerdict}
            onSetExplicitVerdict={onSetExplicitVerdict}
            onSetBody={onSetDraftBody}
          />
        )}
        {activeTab === "inline" && (
          <InlineCommentsTab comments={comments} onToggleSelected={onToggleInlineSelected} onDismiss={onDismissInline} />
        )}
        {activeTab === "safety" && (
          <SafetyTab
            locale={locale}
            safetyState={safetyState}
            preflight={preflight}
            submitMessage={submitMessage}
            onPrepare={onPrepare}
            onOpenConfirmation={() => setConfirmOpen(true)}
          />
        )}
        {activeTab === "runs" && <div className="text-xs text-zinc-500">Select a run above to replace the active draft.</div>}
      </div>
      <PublishConfirmation open={confirmOpen} payload={publishPayload} submitting={submitting} onCancel={() => setConfirmOpen(false)} onConfirm={onConfirm} />
    </aside>
  );
}

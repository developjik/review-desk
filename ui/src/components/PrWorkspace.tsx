import type { ChangedFileContext, Locale, PullRequestContextView, PullRequestQueueItem } from "../lib/view-models";
import { pullRequestRef } from "../lib/view-models";
import { ChangedFileList } from "./ChangedFileList";
import { DiffReader } from "./DiffReader";
import { PrHeader } from "./PrHeader";
import { Button } from "./ui";

type LoadState = "idle" | "loading" | "ready" | "error";

export function PrWorkspace({
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
  if (!selectedPr) return <EmptyWorkspace label="No PR selected" />;
  if (contextState === "loading") return <StatusState title="Loading PR context" message={pullRequestRef(selectedPr)} />;
  if (contextState === "error") {
    return <StatusState title="Could not load PR context" message={contextError ?? "pr_context_failed"} actionLabel="Retry" onAction={onRetryContext} />;
  }
  if (!context) return <EmptyWorkspace label="Select a PR to load changed files and diff." />;

  return (
    <div className="grid min-w-0 grid-rows-[auto_minmax(0,1fr)] overflow-hidden bg-zinc-950">
      <PrHeader locale={locale} context={context} />
      <div className="grid min-h-0 grid-cols-[260px_minmax(0,1fr)] overflow-hidden max-[1023px]:grid-cols-[220px_minmax(0,1fr)]">
        <ChangedFileList locale={locale} files={context.files} selectedFile={selectedFile} viewedFiles={viewedFiles} onSelectFile={onSelectFile} />
        <DiffReader locale={locale} file={selectedFile} viewed={selectedFile ? viewedFiles.has(selectedFile.path) : false} onMarkViewed={onMarkViewed} />
      </div>
    </div>
  );
}

function EmptyWorkspace({ label }: { label: string }) {
  return <div className="grid h-full place-items-center p-6 text-center text-sm text-zinc-500">{label}</div>;
}

function StatusState({ title, message, actionLabel, onAction }: { title: string; message?: string | null; actionLabel?: string; onAction?: () => void }) {
  return (
    <div className="grid h-full place-items-center p-6 text-center">
      <div>
        <h2 className="text-sm font-medium text-zinc-200">{title}</h2>
        {message && <p className="mt-2 text-xs text-zinc-500">{message}</p>}
        {actionLabel && onAction && (
          <Button className="mt-3" variant="ghost" onClick={onAction}>
            {actionLabel}
          </Button>
        )}
      </div>
    </div>
  );
}

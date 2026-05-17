import { RefreshCw } from "lucide-react";
import { Badge, Button } from "./ui";
import { t } from "../lib/i18n";
import {
  prRiskBadges,
  pullRequestRef,
  summarizeQueueItem,
  type InboxSection,
  type Locale,
  type PullRequestQueueItem,
  type Repository,
} from "../lib/view-models";

type QueueFilter = "all" | "requested" | "assigned" | "ci_failed";
type LoadState = "idle" | "loading" | "ready" | "error";

export function AssignedPrQueue({
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
  filter: QueueFilter;
  queueState: LoadState;
  queueError: string | null;
  sections: InboxSection[];
  selectedPr: PullRequestQueueItem | null;
  onSelectRepo: (repo: string | null) => void;
  onSetFilter: (filter: QueueFilter) => void;
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
            <Button key={value} variant={filter === value ? "default" : "ghost"} className="h-8" onClick={() => onSetFilter(value as QueueFilter)}>
              {label}
            </Button>
          ))}
        </div>
      </div>
      {queueState === "loading" && <QueueStatus title="Loading review inbox" />}
      {queueState === "error" && <QueueStatus title="Could not load review inbox" message={queueError ?? "review_queue_failed"} actionLabel="Retry" onAction={onRefresh} />}
      {queueState !== "loading" && queueState !== "error" && totalItems === 0 && <QueueStatus title={t(locale, "inbox.empty")} />}
      {queueState !== "loading" && queueState !== "error" && totalItems > 0 && (
        <div className="h-full overflow-auto pb-10">
          {sections.map((section) => (
            <section key={section.id} className="border-b border-zinc-900">
              <div className="sticky top-0 z-10 flex items-center justify-between bg-zinc-950/95 px-3 py-2 backdrop-blur">
                <div>
                  <h2 className="text-xs font-semibold text-zinc-200">{queueSectionLabel(locale, section.id)}</h2>
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
    <button onClick={onSelect} className={`grid w-full gap-2 border-t border-zinc-900 px-3 py-3 text-left transition hover:bg-zinc-900 ${active ? "bg-zinc-900" : ""}`}>
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

function QueueStatus({ title, message, actionLabel, onAction }: { title: string; message?: string | null; actionLabel?: string; onAction?: () => void }) {
  return (
    <div className="grid place-items-center p-6 text-center">
      <h2 className="text-sm font-medium text-zinc-200">{title}</h2>
      {message && <p className="mt-2 text-xs text-zinc-500">{message}</p>}
      {actionLabel && onAction && (
        <Button className="mt-3" variant="ghost" onClick={onAction}>
          {actionLabel}
        </Button>
      )}
    </div>
  );
}

function queueSectionLabel(locale: Locale, id: InboxSection["id"]): string {
  return t(locale, `inbox.section.${id}`);
}

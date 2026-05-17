import { GitPullRequest } from "lucide-react";
import { Badge } from "./ui";
import { t } from "../lib/i18n";
import type { Locale, PullRequestContextView } from "../lib/view-models";

export function PrHeader({ locale, context }: { locale: Locale; context: PullRequestContextView }) {
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

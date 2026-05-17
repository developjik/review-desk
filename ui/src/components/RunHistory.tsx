import { Badge, Button } from "./ui";
import type { AnalysisRunView } from "../lib/workspace-view-models";

export function RunHistory({
  runs,
  onReplaceDraftFromRun,
}: {
  runs: AnalysisRunView[];
  onReplaceDraftFromRun: (run: AnalysisRunView) => void;
}) {
  return (
    <section className="grid gap-2">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold">Run history</h3>
        <Badge tone="neutral">{runs.length}</Badge>
      </div>
      {runs.length === 0 ? (
        <div className="rounded-md border border-zinc-800 bg-zinc-900 p-3 text-xs text-zinc-500">No analysis runs yet.</div>
      ) : (
        <div className="grid gap-2">
          {runs.map((run) => (
            <article key={run.run_id} className="grid gap-2 rounded-md border border-zinc-800 bg-zinc-900 p-3">
              <div className="flex min-w-0 items-center justify-between gap-2">
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium text-zinc-100">{run.run_id}</div>
                  <div className="text-xs text-zinc-500">{run.mode} · {run.model_id}</div>
                </div>
                <Badge tone={run.status === "draft_ready" ? "green" : run.status === "failed" ? "red" : "blue"}>{run.status}</Badge>
              </div>
              {run.draft_seed_body && (
                <Button variant="ghost" className="h-8 text-xs" onClick={() => onReplaceDraftFromRun(run)}>
                  Replace draft from run
                </Button>
              )}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

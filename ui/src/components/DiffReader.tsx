import { Button } from "./ui";
import { t } from "../lib/i18n";
import { parseUnifiedPatch, type ChangedFileContext, type DiffHunk, type Locale } from "../lib/view-models";

export function DiffReader({
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
            <Button key={hunk.header} variant="ghost" className="h-7 px-2 text-xs" onClick={() => document.getElementById(`hunk-${index}`)?.scrollIntoView({ block: "start" })}>
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
                <div key={`${index}-${lineIndex}`} className={`grid grid-cols-[56px_56px_minmax(0,1fr)] border-b border-zinc-950/80 ${line.type === "add" ? "bg-emerald-950/40 text-emerald-100" : line.type === "delete" ? "bg-red-950/40 text-red-100" : "text-zinc-300"}`}>
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

function EmptyWorkspace({ label }: { label: string }) {
  return <div className="grid h-full place-items-center p-6 text-center text-sm text-zinc-500">{label}</div>;
}

function StatusState({ title, message }: { title: string; message?: string | null }) {
  return (
    <div className="grid h-full place-items-center p-6 text-center">
      <div>
        <h2 className="text-sm font-medium text-zinc-200">{title}</h2>
        {message && <p className="mt-2 text-xs text-zinc-500">{message}</p>}
      </div>
    </div>
  );
}

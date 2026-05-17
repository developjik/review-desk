import { CheckCircle2 } from "lucide-react";
import { Badge, Button } from "./ui";
import type { InlineCommentDraftView } from "../lib/workspace-view-models";

export function InlineCommentsTab({
  comments,
  onToggleSelected,
  onDismiss,
}: {
  comments: InlineCommentDraftView[];
  onToggleSelected: (id: string, selected: boolean) => void;
  onDismiss: (id: string) => void;
}) {
  const visible = comments.filter((comment) => !comment.dismissed);
  return (
    <div className="grid gap-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold">Inline comments</h3>
        <Badge tone="neutral">{visible.length}</Badge>
      </div>
      {visible.length === 0 ? (
        <div className="rounded-md border border-zinc-800 bg-zinc-900 p-3 text-xs text-zinc-500">No inline comments in this draft.</div>
      ) : (
        visible.map((comment) => (
          <article key={comment.id} className="grid gap-2 rounded-md border border-zinc-800 bg-zinc-900 p-3">
            <div className="flex min-w-0 items-center justify-between gap-2">
              <div className="truncate text-sm font-medium text-zinc-100">{comment.path}:{comment.line}</div>
              <Badge tone={comment.mapping_status === "valid" ? "green" : "red"}>{comment.mapping_status}</Badge>
            </div>
            <p className="text-xs leading-5 text-zinc-400">{comment.body}</p>
            <div className="flex items-center justify-between">
              <label className="flex items-center gap-2 text-xs text-zinc-300">
                <input type="checkbox" checked={comment.selected_for_publish} onChange={(event) => onToggleSelected(comment.id, event.target.checked)} />
                Publish
              </label>
              <Button variant="ghost" className="h-8 px-2 text-xs" onClick={() => onDismiss(comment.id)}>
                Dismiss
              </Button>
            </div>
          </article>
        ))
      )}
      {visible.some((comment) => comment.selected_for_publish && comment.mapping_status === "valid") && (
        <Badge tone="green">
          <CheckCircle2 className="mr-1 h-3 w-3" />
          Selected inline comments valid
        </Badge>
      )}
    </div>
  );
}

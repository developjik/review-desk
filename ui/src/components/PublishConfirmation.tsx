import { Send, X } from "lucide-react";
import { Button } from "./ui";
import type { ReviewPublishPayloadView } from "../lib/workspace-view-models";

export function PublishConfirmation({
  open,
  payload,
  submitting,
  onCancel,
  onConfirm,
}: {
  open: boolean;
  payload: ReviewPublishPayloadView | null;
  submitting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  if (!open || !payload) return null;
  const selectedInlineCount = payload.inline_comments.filter((comment) => comment.selected_for_publish && !comment.dismissed).length;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/60 p-4">
      <section className="w-full max-w-xl rounded-md border border-zinc-700 bg-zinc-950 p-4 shadow-2xl">
        <div className="mb-3 flex items-start justify-between gap-3">
          <div>
            <h2 className="text-base font-semibold">Publish review</h2>
            <p className="mt-1 text-xs text-zinc-500">
              {payload.owner}/{payload.repo}#{payload.number} · {payload.event}
            </p>
          </div>
          <Button variant="ghost" className="h-8 px-2" onClick={onCancel} aria-label="Close publish confirmation">
            <X className="h-4 w-4" />
          </Button>
        </div>
        <div className="grid gap-2 rounded-md border border-zinc-800 bg-zinc-900 p-3 text-xs text-zinc-400">
          <span>Head {payload.expected_head_sha.slice(0, 12)}</span>
          <span>Diff {payload.expected_diff_hash.slice(0, 12)}</span>
          <span>Body {payload.body.trim().length} characters</span>
          <span>Inline comments {selectedInlineCount}</span>
        </div>
        <pre className="mt-3 max-h-56 overflow-auto rounded-md border border-zinc-800 bg-zinc-900 p-3 text-xs text-zinc-300">
          {JSON.stringify(payload, null, 2)}
        </pre>
        <div className="mt-4 flex justify-end gap-2">
          <Button variant="ghost" onClick={onCancel}>Cancel</Button>
          <Button onClick={onConfirm} disabled={submitting}>
            <Send className="h-4 w-4" />
            Publish
          </Button>
        </div>
      </section>
    </div>
  );
}

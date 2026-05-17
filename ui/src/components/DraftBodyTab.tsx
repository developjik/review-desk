import { Button } from "./ui";
import { t } from "../lib/i18n";
import type { Locale } from "../lib/view-models";
import type { ReviewEvent } from "../lib/workspace-view-models";

const verdicts: ReviewEvent[] = ["COMMENT", "APPROVE", "REQUEST_CHANGES"];

export function DraftBodyTab({
  locale,
  verdict,
  body,
  explicitVerdict,
  onSetVerdict,
  onSetExplicitVerdict,
  onSetBody,
}: {
  locale: Locale;
  verdict: ReviewEvent;
  body: string;
  explicitVerdict: boolean;
  onSetVerdict: (verdict: ReviewEvent) => void;
  onSetExplicitVerdict: (confirmed: boolean) => void;
  onSetBody: (body: string) => void;
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
        value={body}
        onChange={(event) => onSetBody(event.target.value)}
        placeholder={t(locale, "draft.empty")}
        className="h-[42vh] min-h-64 w-full resize-none rounded-md border border-zinc-800 bg-zinc-900 p-3 text-sm text-zinc-100 outline-none focus:border-sky-500"
        aria-label="Review draft body"
      />
    </div>
  );
}

import { AlertTriangle, CheckCircle2, LockKeyhole, Send, ShieldCheck } from "lucide-react";
import { Badge, Button } from "./ui";
import { t } from "../lib/i18n";
import type { SubmitPreflightView } from "../lib/ipc";
import type { deriveSubmitSafetyState, Locale } from "../lib/view-models";

export function SafetyTab({
  locale,
  safetyState,
  preflight,
  submitMessage,
  onPrepare,
  onOpenConfirmation,
}: {
  locale: Locale;
  safetyState: ReturnType<typeof deriveSubmitSafetyState>;
  preflight: SubmitPreflightView | null;
  submitMessage: string | null;
  onPrepare: () => void;
  onOpenConfirmation: () => void;
}) {
  return (
    <div className="grid gap-3">
      <div className="rounded-md border border-zinc-800 bg-zinc-900 p-3">
        <div className="mb-2 flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm font-medium">
            <ShieldCheck className="h-4 w-4 text-emerald-400" />
            {t(locale, "safety.title")}
          </div>
          <Badge tone={safetyTone(safetyState.status)}>{safetyState.status}</Badge>
        </div>
        <p className="text-xs text-zinc-500">{submitMessage ?? safetyState.label}</p>
      </div>
      {preflight?.blocked_reasons.map((reason) => (
        <Badge key={reason} tone="red">
          <AlertTriangle className="mr-1 h-3 w-3" />
          {t(locale, `reason.${reason}`)}
        </Badge>
      ))}
      {safetyState.status === "ready" && (
        <Badge tone="green">
          <CheckCircle2 className="mr-1 h-3 w-3" />
          {t(locale, "safety.ready")}
        </Badge>
      )}
      <Button variant="ghost" onClick={onPrepare} disabled={!safetyState.canPrepare}>
        <LockKeyhole className="h-4 w-4" />
        {t(locale, "safety.prepare")}
      </Button>
      <Button disabled={!safetyState.canConfirm} onClick={onOpenConfirmation}>
        <Send className="h-4 w-4" />
        {t(locale, "safety.confirm")}
      </Button>
    </div>
  );
}

function safetyTone(status: string): "neutral" | "green" | "amber" | "red" | "blue" {
  if (status === "ready" || status === "submitted") return "green";
  if (status === "blocked") return "red";
  if (status === "dirty") return "amber";
  return "blue";
}

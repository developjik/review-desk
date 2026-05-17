import { Bot, ShieldCheck, TestTube2, Zap } from "lucide-react";
import { Button } from "./ui";
import type { AnalysisRunMode } from "../lib/workspace-view-models";

const quickModes: Array<{ mode: AnalysisRunMode; label: string; icon: typeof Zap }> = [
  { mode: "fast", label: "Fast", icon: Zap },
  { mode: "deep", label: "Deep", icon: Bot },
  { mode: "security", label: "Security", icon: ShieldCheck },
  { mode: "tests", label: "Tests", icon: TestTube2 },
];

export function RunShelf({
  disabled,
  privateConsent,
  onPrivateConsentChange,
  onRunMode,
  onRunCustom,
}: {
  disabled?: boolean;
  privateConsent: boolean;
  onPrivateConsentChange: (accepted: boolean) => void;
  onRunMode: (mode: AnalysisRunMode) => void;
  onRunCustom: () => void;
}) {
  return (
    <div className="grid gap-2">
      <label className="flex items-start gap-2 rounded-md border border-zinc-800 bg-zinc-900 p-3 text-xs text-zinc-300">
        <input
          type="checkbox"
          aria-label="Allow private diff analysis"
          checked={privateConsent}
          onChange={(event) => onPrivateConsentChange(event.target.checked)}
          className="mt-0.5"
        />
        <span>
          <span className="block font-medium text-zinc-100">Allow private diff analysis</span>
          <span className="block text-zinc-500">Required before sending private PR patches to AI analysis.</span>
        </span>
      </label>
      <div className="grid grid-cols-2 gap-2">
        {quickModes.map(({ mode, label, icon: Icon }) => (
          <Button key={mode} variant="ghost" onClick={() => onRunMode(mode)} disabled={disabled}>
            <Icon className="h-4 w-4" />
            {label}
          </Button>
        ))}
      </div>
      <Button onClick={onRunCustom} disabled={disabled}>
        <Bot className="h-4 w-4" />
        Custom
      </Button>
    </div>
  );
}

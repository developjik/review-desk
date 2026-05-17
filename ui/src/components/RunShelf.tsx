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
  onRunMode,
  onRunCustom,
}: {
  disabled?: boolean;
  onRunMode: (mode: AnalysisRunMode) => void;
  onRunCustom: () => void;
}) {
  return (
    <div className="grid gap-2">
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

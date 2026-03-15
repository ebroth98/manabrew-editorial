import { cn } from "@/lib/utils";
import { PHASES } from "./game.constants";

export function MidPhaseStrip({ currentStep }: { currentStep: string }) {
  return (
    <div className="pointer-events-none flex items-center justify-center gap-1 px-2 py-0.5 overflow-x-auto max-w-full">
      {PHASES.map((phase) => (
        <span
          key={phase.id}
          className={cn(
            "text-[10px] px-1.5 py-0.5 rounded border leading-none shrink-0",
            currentStep === phase.id
              ? "bg-primary text-primary-foreground border-primary font-semibold"
              : "bg-background/90 border-border text-muted-foreground",
          )}
          title={phase.label}
        >
          {phase.short}
        </span>
      ))}
    </div>
  );
}

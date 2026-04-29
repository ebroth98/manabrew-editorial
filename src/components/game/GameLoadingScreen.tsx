import { Loader2 } from "lucide-react";
import { getSelectedGameRuntime } from "@/game";
import { useGameStore } from "@/stores/useGameStore";
import { Button } from "@/components/ui/button";

interface GameLoadingScreenProps {
  debugInfo: string;
}

export function GameLoadingScreen({ debugInfo }: GameLoadingScreenProps) {
  const prefetchProgress = useGameStore((s) => s.prefetchProgress);
  const prefetchActive = prefetchProgress != null && prefetchProgress.total > 0;
  const pct = prefetchActive
    ? Math.min(
        100,
        Math.round(
          ((prefetchProgress.loaded + prefetchProgress.failed) / prefetchProgress.total) * 100,
        ),
      )
    : 0;

  return (
    <div className="flex flex-col items-center justify-center h-full gap-4">
      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      <p className="text-muted-foreground">
        {prefetchActive ? "Loading card images…" : "Waiting for game state..."}
      </p>
      {prefetchActive && (
        <div className="flex w-64 flex-col gap-1">
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
            <div
              className="h-full bg-primary transition-[width] duration-150 ease-out"
              style={{ width: `${pct}%` }}
            />
          </div>
          <p className="text-xs text-muted-foreground text-center font-mono">
            {prefetchProgress.loaded + prefetchProgress.failed} / {prefetchProgress.total}
            {prefetchProgress.failed > 0 && (
              <span className="text-destructive"> ({prefetchProgress.failed} failed)</span>
            )}
          </p>
        </div>
      )}
      {debugInfo && <p className="text-xs text-muted-foreground font-mono">{debugInfo}</p>}
      <Button
        variant="outline"
        size="sm"
        onClick={async () => {
          try {
            const runtime = getSelectedGameRuntime();
            const raw = await runtime.api.getPrompt();
            useGameStore.setState({
              debugInfo: `Manual poll: ${JSON.stringify(raw)?.slice(0, 200)}`,
            });
          } catch (e) {
            useGameStore.setState({ debugInfo: `Poll error: ${e}` });
          }
        }}
      >
        Debug: Poll Now
      </Button>
    </div>
  );
}

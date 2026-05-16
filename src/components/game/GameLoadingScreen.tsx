import { Loader2 } from "lucide-react";
import { getSelectedGameRuntime } from "@/game";
import { useGameStore } from "@/stores/useGameStore";
import { Button } from "@/components/ui/button";

interface GameLoadingScreenProps {
  debugInfo: string;
}

export function GameLoadingScreen({ debugInfo }: GameLoadingScreenProps) {
  const isPrefetchingCards = useGameStore((s) => s.isPrefetchingCards);

  return (
    <div className="flex flex-col items-center justify-center h-full gap-4">
      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      <p className="text-muted-foreground">
        {isPrefetchingCards ? "Loading card images…" : "Waiting for game state..."}
      </p>
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

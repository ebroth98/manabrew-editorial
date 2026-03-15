import { invoke } from "@tauri-apps/api/core";
import { useGameStore } from "@/stores/useGameStore";
import { Button } from "@/components/ui/button";

interface GameLoadingScreenProps {
  debugInfo: string;
}

export function GameLoadingScreen({ debugInfo }: GameLoadingScreenProps) {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-4">
      <p className="text-muted-foreground">Waiting for game state...</p>
      {debugInfo && (
        <p className="text-xs text-muted-foreground font-mono">{debugInfo}</p>
      )}
      <Button
        variant="outline"
        size="sm"
        onClick={async () => {
          try {
            const raw = await invoke<unknown>("get_prompt");
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

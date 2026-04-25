import type { Player } from "@/types/openmagic";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface GameOverScreenProps {
  winnerId: string | null | undefined;
  me: Player;
  opponents: Player[];
  turn: number;
  onEndGame: () => void;
}

export function GameOverScreen({ winnerId, me, opponents, turn, onEndGame }: GameOverScreenProps) {
  const isDraw = winnerId == null;
  const didWin = winnerId === me.id;

  return (
    <div className="flex flex-col items-center justify-center h-full gap-4">
      <h2
        className={cn(
          "text-3xl font-bold",
          isDraw ? "text-muted-foreground" : didWin ? "text-success" : "text-destructive",
        )}
      >
        {isDraw ? "Draw!" : didWin ? "You Win!" : "You Lose!"}
      </h2>
      <p className="text-muted-foreground">
        Final life: You {me.life} — {opponents.map((op) => `${op.name} ${op.life}`).join(" · ")}
      </p>
      <p className="text-sm text-muted-foreground">Turn {turn}</p>
      <p className="text-xs text-muted-foreground italic">Returning to menu…</p>
      <Button variant="outline" size="sm" onClick={onEndGame}>
        Return to Menu
      </Button>
    </div>
  );
}

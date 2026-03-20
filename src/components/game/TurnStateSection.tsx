import { cn } from "@/lib/utils";
import { useGameThemeColors, withAlpha } from "./game.theme";

interface TurnStateSectionProps {
  turn: number;
  activePlayerName: string;
  isMyTurn: boolean;
  isMyPriority: boolean;
}

export function TurnStateSection({ turn, activePlayerName, isMyTurn, isMyPriority }: TurnStateSectionProps) {
  const themeColors = useGameThemeColors();

  return (
    <div className="rounded-lg px-2.5 py-2 bg-muted/25">
      <div className="flex items-center gap-1.5">
        <p className="text-sm font-semibold">Turn {turn} -</p>
        <p className="text-sm font-medium" style={{ color: themeColors.activeAction.turnText }}>
          {isMyTurn ? "Your turn" : `${activePlayerName}'s turn`}
        </p>
        {isMyPriority && (
          <span
            className={cn("ml-1 text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0 animate-pulse")}
            style={{
              backgroundColor: withAlpha(themeColors.activeAction.priority, 0.2),
              color: themeColors.activeAction.priority,
            }}
          >
            PRIORITY
          </span>
        )}
      </div>
    </div>
  );
}

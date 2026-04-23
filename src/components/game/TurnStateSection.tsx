import { cn } from "@/lib/utils";
import { withAlpha } from "./game.theme";
import { useTheme } from "@/hooks/useTheme";

interface TurnStateSectionProps {
  turn: number;
  activePlayerName: string;
  isMyTurn: boolean;
  isMyPriority: boolean;
}

export function TurnStateSection({ turn, activePlayerName, isMyTurn, isMyPriority }: TurnStateSectionProps) {
  const themeColors = useTheme().game;

  return (
    <div className="rounded-lg px-2.5 py-2 bg-muted/25">
      <div className="flex items-center gap-1.5">
        <p className="text-sm font-semibold">Turn {turn} -</p>
        <p className="text-sm font-medium" style={{ color: themeColors.activeAction.active }}>
          {isMyTurn ? "Your turn" : `${activePlayerName}'s turn`}
        </p>
        {isMyPriority && (
          <span
            className={cn("ml-1 text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0 animate-pulse")}
            style={{
              backgroundColor: withAlpha(themeColors.activeAction.active, 0.2),
              color: themeColors.activeAction.active,
            }}
          >
            PRIORITY
          </span>
        )}
      </div>
    </div>
  );
}

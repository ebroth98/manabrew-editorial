import type { Player } from "@/types/xmage";
import { cn } from "@/lib/utils";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Heart, Layers, Sword } from "lucide-react";
import { ManaPool } from "./ManaPool";
import { getAvatarColor, getInitials } from "../game.utils";

interface PlayerPanelProps {
  player: Player;
  isOpponent: boolean;
  isActiveTurn?: boolean;
  isPriorityPlayer?: boolean;
  isTargetable?: boolean;
  onTarget?: () => void;
  isFlashing?: boolean;
  onOpenCommandZone?: () => void;
  commandZoneCount?: number;
}

export function PlayerPanel({
  player,
  isOpponent,
  isActiveTurn,
  isPriorityPlayer,
  isTargetable,
  onTarget,
  isFlashing,
  onOpenCommandZone,
  commandZoneCount = 0,
}: PlayerPanelProps) {
  const totalCmdDmg = Object.values(player.commanderDamage ?? {}).reduce(
    (a, b) => a + b,
    0,
  );

  return (
    <div
      data-player-id={player.id}
      className={cn(
        "flex items-center gap-3 px-3 py-2 rounded-lg border border-border bg-card text-sm transition-colors",
        isPriorityPlayer &&
          !isTargetable &&
          "bg-purple-50/50 dark:bg-purple-950/15 shadow-[inset_0_0_0_2px_rgba(168,85,247,0.95)]",
        isActiveTurn &&
          !isTargetable &&
          !isPriorityPlayer &&
          (isOpponent
            ? "bg-orange-50/40 dark:bg-orange-950/10 shadow-[inset_0_0_0_2px_rgba(251,146,60,0.95)]"
            : "bg-green-50/40 dark:bg-green-950/10 shadow-[inset_0_0_0_2px_rgba(34,197,94,0.95)]"),
        isTargetable &&
          "cursor-pointer hover:bg-red-50 dark:hover:bg-red-950/30 shadow-[inset_0_0_0_2px_rgba(248,113,113,0.95)]",
        isFlashing && "animate-player-turn-flash",
      )}
      onClick={isTargetable ? onTarget : undefined}
      title={isTargetable ? `Target ${player.name}` : undefined}
    >
      <Avatar className="h-8 w-8 shrink-0">
        <AvatarFallback
          className={cn("text-xs font-bold", getAvatarColor(player.name))}
        >
          {getInitials(player.name)}
        </AvatarFallback>
      </Avatar>
      <div className="flex items-center gap-1 shrink-0">
        <Heart className="h-3.5 w-3.5 text-red-500" />
        <span className="font-bold">{player.life}</span>
      </div>
      <div className="font-semibold truncate min-w-0">{player.name}</div>
      {isOpponent && isActiveTurn && (
        <span
          className={cn(
            "text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0",
            isOpponent
              ? "bg-orange-100 text-orange-700 dark:bg-orange-950/40 dark:text-orange-400"
              : "bg-green-100 text-green-700 dark:bg-green-950/40 dark:text-green-400",
          )}
        >
          {isOpponent ? "THEIR TURN" : "YOUR TURN"}
        </span>
      )}
      {isOpponent && isPriorityPlayer && (
        <span className="text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0 bg-purple-100 text-purple-700 dark:bg-purple-950/40 dark:text-purple-300 animate-pulse">
          PRIORITY
        </span>
      )}
      {isTargetable && (
        <Badge
          variant="destructive"
          className="text-xs h-5 px-1 animate-pulse shrink-0"
        >
          TARGET
        </Badge>
      )}
      {player.poison > 0 && (
        <Badge variant="destructive" className="text-xs h-5 px-1 shrink-0">
          {player.poison} ☠
        </Badge>
      )}
      {(player.energyCounters ?? 0) > 0 && (
        <Badge
          variant="outline"
          className="text-xs h-5 px-1 text-yellow-600 border-yellow-400 shrink-0"
          title="Energy counters"
        >
          {player.energyCounters} ⚡
        </Badge>
      )}
      {totalCmdDmg > 0 && (
        <Badge
          variant="outline"
          className="text-xs h-5 px-1 text-orange-600 border-orange-400 shrink-0"
          title={`Commander damage received: ${totalCmdDmg}`}
        >
          ⚔{totalCmdDmg} CMD
        </Badge>
      )}
      {commandZoneCount > 0 && (
        <button
          className={cn(
            "inline-flex items-center gap-1 px-1.5 py-0.5 rounded border text-xs shrink-0",
            onOpenCommandZone
              ? "border-border text-muted-foreground hover:text-foreground hover:bg-muted/40"
              : "border-border text-muted-foreground/80",
          )}
          onClick={onOpenCommandZone}
          disabled={!onOpenCommandZone}
          title="Command Zone"
        >
          <Sword className="h-3 w-3" />
          <span>{commandZoneCount}</span>
        </button>
      )}
      <div className="flex items-center gap-1 text-xs text-muted-foreground shrink-0">
        <Layers className="h-3 w-3" />
        <span>{player.handCount}</span>
      </div>
      {!isOpponent && (
        <div className="ml-auto">
          <ManaPool pool={player.manaPool} />
        </div>
      )}
    </div>
  );
}

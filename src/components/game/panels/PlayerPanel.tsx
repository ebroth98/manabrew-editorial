import type { CSSProperties } from "react";
import type { Player } from "@/types/openmagic";
import { cn } from "@/lib/utils";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Heart, Layers, Sword } from "lucide-react";
import { getAvatarColor, getInitials } from "../game.utils";
import { useGameThemeColors } from "../game.theme";

interface PlayerPanelProps {
  player: Player;
  isOpponent: boolean;
  className?: string;
  verticalAlign?: "top" | "bottom";
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
  className,
  verticalAlign = "top",
  isActiveTurn,
  isPriorityPlayer: _isPriorityPlayer,
  isTargetable,
  onTarget,
  isFlashing,
  onOpenCommandZone,
  commandZoneCount = 0,
}: PlayerPanelProps) {
  const DEV_SHOW_ALL_STATS = false;
  const themeColors = useGameThemeColors();

  const totalCmdDmg = Object.values(player.commanderDamage ?? {}).reduce(
    (a, b) => a + b,
    0,
  );

  const avatarRingColor = isOpponent
    ? themeColors.activeAction.opponentTurnRing
    : themeColors.activeAction.myTurnRing;

  const isTop = verticalAlign === "top";

  const statsBar = (
    <div className="flex items-center gap-1 bg-black/60 backdrop-blur-sm rounded-full pl-10 pr-2 py-0.5 shadow-sm">
      <span className="inline-flex items-center gap-1 text-white">
        <Heart className="h-3 w-3 text-red-400" />
        <span className="font-extrabold text-[11px] leading-none tabular-nums">{player.life}</span>
      </span>
      <span className="text-white/20">|</span>
      <span className="inline-flex items-center gap-1 text-white/90">
        <Layers className="h-3 w-3 text-sky-300" />
        <span className="font-bold text-[11px] leading-none tabular-nums">{player.handCount}</span>
      </span>
      {(DEV_SHOW_ALL_STATS || player.poison > 0) && (
        <>
          <span className="text-white/20">|</span>
          <span className="inline-flex items-center gap-1 text-red-300 font-bold text-[11px]">
            ☠<span className="tabular-nums">{player.poison || 0}</span>
          </span>
        </>
      )}
      {(DEV_SHOW_ALL_STATS || (player.energyCounters ?? 0) > 0) && (
        <>
          <span className="text-white/20">|</span>
          <span className="inline-flex items-center gap-1 text-yellow-300 font-bold text-[11px]">
            ⚡<span className="tabular-nums">{player.energyCounters ?? 0}</span>
          </span>
        </>
      )}
      {(DEV_SHOW_ALL_STATS || totalCmdDmg > 0) && (
        <>
          <span className="text-white/20">|</span>
          <span className="inline-flex items-center gap-1 text-orange-300 font-bold text-[11px]">
            ⚔<span className="tabular-nums">{totalCmdDmg || 0}</span>
          </span>
        </>
      )}
      {(DEV_SHOW_ALL_STATS || commandZoneCount > 0) && (
        <>
          <span className="text-white/20">|</span>
          <button
            className={cn(
              "inline-flex items-center gap-1 font-bold text-[11px]",
              onOpenCommandZone
                ? "text-sky-300 hover:text-sky-200"
                : "text-muted-foreground/80",
            )}
            onClick={onOpenCommandZone}
            disabled={!onOpenCommandZone}
            title="Command Zone"
          >
            <Sword className="h-3 w-3" />
            <span className="tabular-nums">{commandZoneCount}</span>
          </button>
        </>
      )}
    </div>
  );

  return (
    <div
      data-player-id={player.id}
      className={cn(
        "relative transition-colors",
        isTargetable && "cursor-pointer",
        isFlashing && "animate-player-turn-flash",
        className,
      )}
      onClick={isTargetable ? onTarget : undefined}
      title={isTargetable ? `Target ${player.name}` : undefined}
    >
      {/* Avatar */}
      <div className="relative z-10 h-10 w-10 shrink-0 p-0.5">
        <div className="relative group/avatar h-full w-full">
          <Avatar
            className={cn(
              "h-full w-full",
              isTargetable && "ring-2 ring-red-400/90",
            )}
            style={
              isActiveTurn
                ? ({ boxShadow: `0 0 0 2px ${avatarRingColor}` } as CSSProperties)
                : undefined
            }
          >
            <AvatarFallback
              className={cn("text-xs font-bold", getAvatarColor(player.name))}
            >
              {getInitials(player.name)}
            </AvatarFallback>
          </Avatar>
          <span className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-[60%] hidden group-hover/avatar:block whitespace-nowrap rounded bg-black/85 px-1.5 py-0.5 text-[10px] font-semibold text-white pointer-events-none z-40">
            {player.name}
          </span>
        </div>
      </div>

      {/* Stats bar — anchored to bottom (player) or top (opponent), starts behind avatar */}
      <div
        className={cn(
          "absolute left-2",
          isTop ? "top-0" : "bottom-0",
        )}
      >
        {statsBar}
      </div>
    </div>
  );
}

import type { CSSProperties, ReactNode } from "react";
import type { Player } from "@/types/openmagic";
import { cn } from "@/lib/utils";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Heart, Layers, Sword } from "lucide-react";
import { ManaPool as ManaPoolDisplay } from "./ManaPool";
import { getAvatarColor, getInitials } from "../game.utils";
import { useGameThemeColors, withAlpha } from "../game.theme";

interface StatBadgeProps {
  label: string;
  children: ReactNode;
  asButton?: boolean;
  className?: string;
  style?: CSSProperties;
  onClick?: () => void;
  disabled?: boolean;
}

function StatBadge({
  label,
  children,
  asButton = false,
  className,
  style,
  onClick,
  disabled,
}: StatBadgeProps) {
  const baseClassName = cn(
    "relative inline-flex items-center",
    asButton && "disabled:opacity-80",
    className,
  );

  const tooltip = (
    <span className="pointer-events-none absolute left-1/2 top-full z-40 mt-1 -translate-x-1/2 whitespace-nowrap rounded bg-black/85 px-1.5 py-0.5 text-xs font-semibold text-white opacity-0 transition-opacity duration-150 delay-0 group-hover/stat:delay-300 group-hover/stat:opacity-100">
      {label}
    </span>
  );

  if (asButton) {
    return (
      <button
        className={cn(baseClassName, "group/stat")}
        style={style}
        onClick={onClick}
        disabled={disabled}
      >
        {children}
        {tooltip}
      </button>
    );
  }

  return (
    <span className={cn(baseClassName, "group/stat")} style={style}>
      {children}
      {tooltip}
    </span>
  );
}

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
  isOpponent: _isOpponent,
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

  const avatarRingColor = themeColors.activeAction.active;

  const isTop = verticalAlign === "top";

  const hasMana = Object.values(player.manaPool).some((v) => v > 0);

  const statsBar = (
      <div
        className="flex items-center gap-1 backdrop-blur-sm shadow-sm pl-10 pr-2 py-0.5 rounded-full"
        style={{
          backgroundColor: withAlpha(themeColors.promptAction.cancel, 0.58),
        }}
      >
        <StatBadge label="Life Total">
          <span className="inline-flex items-center gap-1 text-white">
            <Heart
              className="h-3 w-3"
              style={{ color: themeColors.promptAction.attackAction }}
            />
            <span className="font-extrabold text-[11px] leading-none tabular-nums">
              {player.life}
            </span>
          </span>
        </StatBadge>
        <span className="text-white/20">|</span>
        <StatBadge label="Cards in Hand">
          <span className="inline-flex items-center gap-1 text-white/90">
            <Layers
              className="h-3 w-3"
              style={{ color: themeColors.promptAction.defenseAction }}
            />
            <span className="font-bold text-[11px] leading-none tabular-nums">
              {player.handCount}
            </span>
          </span>
        </StatBadge>
        {(DEV_SHOW_ALL_STATS || player.poison > 0) && (
          <>
            <span className="text-white/20">|</span>
            <StatBadge label="Poison Counters">
              <span className="inline-flex items-center gap-1 font-bold text-[11px] text-poison">
                ☠<span className="tabular-nums">{player.poison || 0}</span>
              </span>
            </StatBadge>
          </>
        )}
        {(DEV_SHOW_ALL_STATS || (player.energyCounters ?? 0) > 0) && (
          <>
            <span className="text-white/20">|</span>
            <StatBadge label="Energy Counters">
              <span
                className="inline-flex items-center gap-1 font-bold text-[11px]"
                style={{ color: withAlpha(themeColors.activeAction.active, 0.9) }}
              >
                ⚡
                <span className="tabular-nums">{player.energyCounters ?? 0}</span>
              </span>
            </StatBadge>
          </>
        )}
        {(DEV_SHOW_ALL_STATS || totalCmdDmg > 0) && (
          <>
            <span className="text-white/20">|</span>
            <StatBadge label="Commander Damage Taken">
              <span
                className="inline-flex items-center gap-1 font-bold text-[11px]"
                style={{ color: withAlpha(themeColors.activeAction.active, 0.9) }}
              >
                ⚔<span className="tabular-nums">{totalCmdDmg || 0}</span>
              </span>
            </StatBadge>
          </>
        )}
        {(DEV_SHOW_ALL_STATS || commandZoneCount > 0) && (
          <>
            <span className="text-white/20">|</span>
            <StatBadge
              label="Command Zone"
              asButton
              className={cn(
                "font-bold text-[11px]",
                onOpenCommandZone ? "" : "text-muted-foreground/80",
              )}
              style={
                onOpenCommandZone
                  ? { color: themeColors.promptAction.defenseAction }
                  : undefined
              }
              onClick={onOpenCommandZone}
              disabled={!onOpenCommandZone}
            >
              <Sword className="h-3 w-3" />
              <span className="tabular-nums">{commandZoneCount}</span>
            </StatBadge>
          </>
        )}
      </div>
  );

  const targetableColor = withAlpha(themeColors.promptAction.attackAction, 0.9);

  return (
    <div
      data-player-id={player.id}
      className={cn(
        "relative transition-colors rounded-full",
        isTargetable && "cursor-pointer animate-player-targetable-pulse",
        isFlashing && "animate-player-turn-flash",
        className,
      )}
      style={
        isTargetable
          ? ({ "--targetable-color": targetableColor } as CSSProperties)
          : undefined
      }
      onClick={isTargetable ? onTarget : undefined}
      title={isTargetable ? `Target ${player.name}` : undefined}
    >
      {/* Avatar */}
      <div className="relative z-10 h-10 w-10 shrink-0 p-0.5">
        <div className="relative group/avatar h-full w-full">
          <Avatar
            className={cn("h-full w-full", isTargetable && "ring-2")}
            style={{
              ...(isTargetable
                ? ({ "--tw-ring-color": targetableColor } as CSSProperties)
                : {}),
              ...(isActiveTurn
                ? ({
                    boxShadow: `0 0 0 2px ${avatarRingColor}`,
                  } as CSSProperties)
                : {}),
            }}
          >
            <AvatarFallback
              className={cn("text-xs font-bold", getAvatarColor(player.name))}
            >
              {getInitials(player.name)}
            </AvatarFallback>
          </Avatar>
          <span className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-[60%] hidden group-hover/avatar:block whitespace-nowrap rounded bg-black/85 px-1.5 py-0.5 text-xs font-semibold text-white pointer-events-none z-40">
            {player.name}
          </span>
        </div>
      </div>

      {/* Stats bar + mana pool — anchored to bottom (player) or top (opponent) */}
      <div className={cn("absolute left-2 flex items-center gap-0.5", isTop ? "top-0" : "bottom-0")}>
        {statsBar}
        {hasMana && (
          <StatBadge label="Mana Pool">
            <div
              className="flex items-center rounded-full px-1.5 py-0.5 shrink-0 backdrop-blur-sm shadow-sm"
              style={{
                backgroundColor: withAlpha(themeColors.promptAction.cancel, 0.58),
              }}
            >
              <ManaPoolDisplay pool={player.manaPool} />
            </div>
          </StatBadge>
        )}
      </div>
    </div>
  );
}

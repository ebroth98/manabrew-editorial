import type { CSSProperties } from "react";
import type { Player } from "@/types/openmagic";
import { cn } from "@/lib/utils";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Heart } from "lucide-react";
import { getInitials } from "../game.utils";
import { withAlpha } from "../game.theme";
import { useTheme } from "@/hooks/useTheme";
import { BadgeOrbit, type OrbitBadge } from "./BadgeOrbit";

export interface PlayerAvatarProps {
  player: Player;
  badges: OrbitBadge[];
  /** Per-seat theme colour. Drives the avatar fill and the active-turn
   *  glow. Resolved by the parent from `themeColors.playerColors[seat]`. */
  seatColor: string;
  isActiveTurn?: boolean;
  isPriorityPlayer?: boolean;
  isTargetable?: boolean;
  isSelectedTarget?: boolean;
  onTarget?: () => void;
  isFlashing?: boolean;
  className?: string;
}

const AVATAR_PX = 72;

export function PlayerAvatar({
  player,
  badges,
  seatColor,
  isActiveTurn,
  isTargetable,
  isSelectedTarget,
  onTarget,
  isFlashing,
  className,
}: PlayerAvatarProps) {
  const theme = useTheme();
  const themeColors = theme.game;
  const fontSizes = theme.fontSizes;
  const targetableColor = withAlpha(themeColors.promptAction.attackAction, 0.9);
  const selectedTargetColor = themeColors.promptAction.attackAction;

  const ringStyle: CSSProperties = isSelectedTarget
    ? { boxShadow: `0 0 0 3px ${selectedTargetColor}, 0 0 14px ${withAlpha(selectedTargetColor, 0.7)}` }
    : isTargetable
      ? { boxShadow: `0 0 0 2px ${targetableColor}` }
      : isActiveTurn
        ? { boxShadow: `0 0 0 2px ${seatColor}, 0 0 16px ${withAlpha(seatColor, 0.75)}` }
        : {};

  return (
    <div
      className={cn(
        "relative inline-flex flex-col items-center gap-0.5",
        className,
      )}
      data-player-id={player.id}
    >
      <div
        className="relative"
        style={{ width: AVATAR_PX, height: AVATAR_PX }}
        onClick={isTargetable ? onTarget : undefined}
        title={isTargetable ? `Target ${player.name}` : player.name}
      >
        <Avatar
          className={cn(
            "h-full w-full",
            isTargetable && "cursor-pointer animate-player-targetable-pulse",
            isFlashing && "animate-player-turn-flash",
          )}
          style={{
            ...ringStyle,
            ...(isFlashing
              ? ({ "--turn-flash-color": seatColor } as CSSProperties)
              : {}),
            ...(isTargetable || isSelectedTarget
              ? ({ "--targetable-color": targetableColor, "--tw-ring-color": selectedTargetColor } as CSSProperties)
              : {}),
          }}
        >
          <AvatarFallback
            className="font-bold text-white"
            style={{ backgroundColor: seatColor, fontSize: fontSizes.avatarInitials }}
          >
            {getInitials(player.name)}
          </AvatarFallback>
        </Avatar>

        <div className="pointer-events-none absolute left-1/2 -bottom-2 -translate-x-1/2 z-30">
          <span
            className="flex items-center gap-1 rounded-full px-2 py-0.5 text-white shadow ring-1 ring-black/50"
            style={{ backgroundColor: "rgba(0, 0, 0, 0.82)" }}
          >
            <Heart
              className="h-3.5 w-3.5"
              style={{ color: themeColors.life, fill: themeColors.life }}
            />
            <span
              className="font-extrabold leading-none tabular-nums"
              style={{ fontSize: fontSizes.life }}
            >
              {player.life}
            </span>
          </span>
        </div>

        <BadgeOrbit badges={badges} avatarSize={AVATAR_PX} />
      </div>
    </div>
  );
}

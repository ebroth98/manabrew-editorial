import type { CSSProperties } from "react";
import { cn } from "@/lib/utils";
import { useGameThemeColors, withAlpha } from "@/components/game/game.theme";

type OverlayVariant = "tap" | "untap" | "choosable" | "pending" | "attacking";

interface CardOverlayButtonProps {
  variant: OverlayVariant;
  onClick: () => void;
  title?: string;
  label?: string;
  /** Stop mousedown propagation (needed in FreeBattlefield to prevent drag) */
  stopMouseDown?: boolean;
}

export function CardOverlayButton({ variant, onClick, title, label, stopMouseDown }: CardOverlayButtonProps) {
  const themeColors = useGameThemeColors();

  const variantColorMap: Record<OverlayVariant, string> = {
    tap: themeColors.activeAction.active,
    untap: themeColors.promptAction.cancel,
    choosable: themeColors.promptAction.defenseAction,
    pending: themeColors.promptAction.passAction,
    attacking: themeColors.promptAction.attackAction,
  };

  const baseColor = variantColorMap[variant];
  const buttonStyle: CSSProperties = {
    backgroundColor: withAlpha(baseColor, 0.2),
    borderColor: baseColor,
  };
  const labelStyle: CSSProperties = {
    backgroundColor: withAlpha(baseColor, 0.92),
    color: "#fff",
  };

  return (
    <button
      className={cn(
        "absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 border-2 transition-opacity",
        label && "flex items-end justify-center pb-1",
      )}
      style={buttonStyle}
      onClick={onClick}
      onMouseDown={stopMouseDown ? (e) => e.stopPropagation() : undefined}
      title={title}
    >
      {label && (
        <span className="text-[9px] font-bold px-1 rounded leading-none" style={labelStyle}>
          {label}
        </span>
      )}
    </button>
  );
}

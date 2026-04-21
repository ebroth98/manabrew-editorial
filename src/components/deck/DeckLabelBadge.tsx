import { cn } from "@/lib/utils";
import { useGameThemeColors } from "@/components/game/game.theme";
import type { DeckLabel } from "@/types/openmagic";

/** Return the theme's shadow (dark) or neutral (light) colour depending on
 *  which produces better contrast against the supplied hex background. */
export function getContrastColor(hex: string, darkInk: string, lightInk: string): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return (0.299 * r + 0.587 * g + 0.114 * b) / 255 > 0.5 ? darkInk : lightInk;
}

type Size = "sm" | "md";

const SIZE_CLASSES: Record<Size, string> = {
  sm: "text-[8px] px-1 py-0",
  md: "text-[10px] px-1.5 py-0.5",
};

interface DeckLabelBadgeProps {
  label: DeckLabel;
  size?: Size;
  className?: string;
}

export function DeckLabelBadge({ label, size = "sm", className }: DeckLabelBadgeProps) {
  const themeColors = useGameThemeColors();
  return (
    <span
      className={cn("rounded-full font-medium border leading-tight", SIZE_CLASSES[size], className)}
      style={label.color
        ? {
            backgroundColor: label.color,
            color: getContrastColor(label.color, themeColors.canvas.shadow, themeColors.canvas.neutral),
            borderColor: label.color,
          }
        : { backgroundColor: "hsl(var(--muted))", color: "hsl(var(--muted-foreground))", borderColor: "hsl(var(--border))" }
      }
    >
      {label.name}
    </span>
  );
}

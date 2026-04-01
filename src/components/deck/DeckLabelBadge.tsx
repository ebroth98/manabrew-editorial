import { cn } from "@/lib/utils";
import type { DeckLabel } from "@/types/openmagic";

/** Returns black or white depending on background luminance. */
export function getContrastColor(hex: string): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return (0.299 * r + 0.587 * g + 0.114 * b) / 255 > 0.5 ? "#000000" : "#ffffff";
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
  return (
    <span
      className={cn("rounded-full font-medium border leading-tight", SIZE_CLASSES[size], className)}
      style={label.color
        ? { backgroundColor: label.color, color: getContrastColor(label.color), borderColor: label.color }
        : { backgroundColor: "hsl(var(--muted))", color: "hsl(var(--muted-foreground))", borderColor: "hsl(var(--border))" }
      }
    >
      {label.name}
    </span>
  );
}

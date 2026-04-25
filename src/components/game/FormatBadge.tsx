import { getFormat } from "@/lib/formats";
import { cn } from "@/lib/utils";

/** Subtle text-only color accents — no loud backgrounds. */
const COLOR_CLASSES: Record<string, string> = {
  blue: "text-blue-500",
  amber: "text-amber-500",
  emerald: "text-emerald-500",
  rose: "text-rose-500",
  slate: "text-slate-400",
  zinc: "text-zinc-400",
  purple: "text-purple-500",
  teal: "text-teal-500",
  orange: "text-orange-500",
  sky: "text-sky-500",
  indigo: "text-indigo-500",
};

interface FormatBadgeProps {
  formatId: string;
  className?: string;
}

export function FormatBadge({ formatId, className }: FormatBadgeProps) {
  const format = getFormat(formatId);
  if (!format) return null;
  const textColor = COLOR_CLASSES[format.badgeColor] ?? "text-muted-foreground";
  return (
    <span
      className={cn(
        "inline-flex items-center px-1 py-px rounded text-[10px] font-semibold uppercase tracking-wide bg-muted/60",
        textColor,
        className,
      )}
      title={format.description}
    >
      {format.shortName}
    </span>
  );
}

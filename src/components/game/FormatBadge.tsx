import { getFormat } from "@/lib/formats";
import { cn } from "@/lib/utils";

const COLOR_CLASSES: Record<string, string> = {
  blue: "bg-blue-100 text-blue-800 border-blue-300",
  purple: "bg-purple-100 text-purple-800 border-purple-300",
};

interface FormatBadgeProps {
  formatId: string;
  className?: string;
}

export function FormatBadge({ formatId, className }: FormatBadgeProps) {
  const format = getFormat(formatId);
  if (!format) return null;
  const colorClasses =
    COLOR_CLASSES[format.badgeColor] ??
    "bg-gray-100 text-gray-800 border-gray-300";
  return (
    <span
      className={cn(
        "inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium border",
        colorClasses,
        className
      )}
      title={format.description}
    >
      {format.shortName}
    </span>
  );
}

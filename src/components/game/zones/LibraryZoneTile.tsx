import { cn } from "@/lib/utils";
import { useGameThemeColors, withAlpha } from "@/components/game/game.theme";

interface LibraryZoneTileProps {
  count: number;
  onClick?: () => void;
  label?: string;
}

export function LibraryZoneTile({
  count,
  onClick,
  label = "Lib",
}: LibraryZoneTileProps) {
  const themeColors = useGameThemeColors();
  const base = themeColors.promptAction.defenseAction;
  const accent = themeColors.activeAction.active;

  return (
    <div className="flex flex-col items-center gap-0.5">
      <div className="relative h-16 w-10">
        <span
          className="absolute left-1 top-1 h-14 w-8 rounded-md border"
          style={{ borderColor: withAlpha(accent, 0.35), backgroundColor: withAlpha(base, 0.2) }}
        />
        <span
          className="absolute left-0.5 top-0.5 h-14 w-8 rounded-md border"
          style={{ borderColor: withAlpha(accent, 0.55), backgroundColor: withAlpha(base, 0.34) }}
        />
        <button
          className={cn(
            "absolute left-0 top-0 h-14 w-8 rounded-md text-card-foreground transition-colors",
            "border-2",
            onClick ? "" : "opacity-95",
          )}
          style={{
            backgroundColor: withAlpha(base, onClick ? 0.55 : 0.48),
            borderColor: withAlpha(accent, 0.8),
            boxShadow: `inset 0 0 0 1px ${withAlpha(base, 0.35)}`,
          }}
          onClick={onClick}
          disabled={!onClick}
          title="Library"
        >
          <span
            className="absolute inset-[4px] rounded-[4px] border"
            style={{ borderColor: withAlpha(base, 0.3), backgroundColor: withAlpha(base, 0.3) }}
          />
          <span className="relative z-10 text-base font-bold leading-none text-white">
            {count}
          </span>
        </button>
      </div>
      <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
        {label}
      </span>
    </div>
  );
}

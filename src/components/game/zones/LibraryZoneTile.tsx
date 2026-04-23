import { cn } from "@/lib/utils";
import { useGameFontSizes, useGameThemeColors, withAlpha } from "@/components/game/game.theme";
import { CARD_BACK_IMAGE_URL } from "@/components/game/game.constants";

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
  const fontSizes = useGameFontSizes();
  const ringColor = themeColors.activeAction.active;
  const empty = count === 0;

  return (
    <div className="flex flex-col items-center gap-0.5">
      <button
        type="button"
        className={cn(
          "relative h-[100px] w-[72px] overflow-hidden rounded-md transition-colors",
          "border-2",
          onClick ? "hover:brightness-110 cursor-pointer" : "opacity-95",
        )}
        style={{
          borderColor: withAlpha(ringColor, 0.8),
          boxShadow: `0 1px 4px ${withAlpha(ringColor, 0.25)}`,
        }}
        onClick={onClick}
        disabled={!onClick}
        title="Library"
      >
        {!empty && (
          <img
            src={CARD_BACK_IMAGE_URL}
            alt=""
            loading="eager"
            className="absolute inset-0 h-full w-full object-cover pointer-events-none select-none"
            draggable={false}
          />
        )}
        <span
          className="absolute bottom-0 left-0 right-0 flex justify-center py-0.5 font-extrabold leading-none tabular-nums text-white"
          style={{ backgroundColor: "rgba(0, 0, 0, 0.62)", fontSize: fontSizes.zoneCount }}
        >
          {count}
        </span>
      </button>
      <span
        className="uppercase tracking-wide text-muted-foreground"
        style={{ fontSize: fontSizes.zoneLabel }}
      >
        {label}
      </span>
    </div>
  );
}

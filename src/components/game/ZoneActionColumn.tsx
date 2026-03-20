import { Archive } from "lucide-react";

import { cn } from "@/lib/utils";
import { useGameThemeColors, withAlpha } from "@/components/game/game.theme";
import { LibraryZoneTile } from "@/components/game/zones";
import type { ZonePanelItem } from "@/stores/usePreferencesStore";

export interface ZoneActionColumnProps {
  libraryCount: number;
  graveyardCount: number;
  exileCount: number;
  order?: ZonePanelItem[];
  onOpenLibrary?: () => void;
  onOpenGraveyard?: () => void;
  onOpenExile?: () => void;
  hasPlayableInGraveyard?: boolean;
  hasPlayableInExile?: boolean;
}

function ZoneCountTile({
  count,
  label,
  title,
  onClick,
  highlighted,
  highlightColor,
}: {
  count: number;
  label: string;
  title: string;
  onClick?: () => void;
  highlighted?: boolean;
  highlightColor: string;
}) {
  return (
    <div className="flex flex-col items-center gap-0.5">
      <button
        className={cn(
          "h-16 w-10 rounded-md border-2 border-dashed text-xs leading-none transition-colors",
          "flex flex-col items-center justify-center gap-1",
          highlighted
            ? "animate-pulse"
            : onClick
              ? "border-muted-foreground/45 text-muted-foreground bg-muted/10 hover:border-primary hover:text-foreground"
              : "border-muted-foreground/35 text-muted-foreground/80 bg-muted/5",
        )}
        style={
          highlighted
            ? {
                borderColor: highlightColor,
                color: highlightColor,
                backgroundColor: withAlpha(highlightColor, 0.15),
              }
            : undefined
        }
        onClick={onClick}
        disabled={!onClick}
        title={title}
      >
        <Archive className="h-3.5 w-3.5" />
        <span className="text-lg font-bold leading-none">{count}</span>
      </button>
      <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
        {label}
      </span>
    </div>
  );
}

export function ZoneActionColumn({
  libraryCount,
  graveyardCount,
  exileCount,
  order = ["library", "graveyard", "exile"],
  onOpenLibrary,
  onOpenGraveyard,
  onOpenExile,
  hasPlayableInGraveyard,
  hasPlayableInExile,
}: ZoneActionColumnProps) {
  const themeColors = useGameThemeColors();
  const items = {
    library: (
      <LibraryZoneTile key="library" count={libraryCount} onClick={onOpenLibrary} />
    ),
    graveyard: (
      <ZoneCountTile
        key="graveyard"
        count={graveyardCount}
        label="GY"
        title="Graveyard"
        onClick={onOpenGraveyard}
        highlighted={hasPlayableInGraveyard}
        highlightColor={themeColors.activeAction.active}
      />
    ),
    exile: (
      <ZoneCountTile
        key="exile"
        count={exileCount}
        label="Exile"
        title="Banish"
        onClick={onOpenExile}
        highlighted={hasPlayableInExile}
        highlightColor={themeColors.activeAction.active}
      />
    ),
  } as const;

  return (
    <div className="w-12 shrink-0 flex flex-col items-center gap-1.5 py-0.5">
      {order.map((item) => items[item])}
    </div>
  );
}

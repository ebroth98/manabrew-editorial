import { memo } from "react";
import { cn } from "@/lib/utils";
import { extractManaLetters } from "@/components/game/manaUtils";
import { manaSymbolUrl } from "@/components/game/game.utils";

export const MANA_COLORS: Record<string, string> = {
  W: "rgba(248, 246, 216, 0.45)", // White
  U: "rgba(193, 215, 233, 0.45)", // Blue
  B: "rgba(186, 177, 171, 0.45)", // Black
  R: "rgba(235, 159, 130, 0.45)", // Red
  G: "rgba(196, 211, 202, 0.45)", // Green
  C: "rgba(204, 202, 199, 0.45)", // Colorless
};

/** A button with a mana symbol for tapping a dual land for a specific color, styled to fill card sections. */
export const ManaAbilityTapButton = memo(function ManaAbilityTapButton({
  description,
  onClick,
  small = false,
  className,
}: {
  description: string;
  onClick: () => void;
  small?: boolean;
  className?: string;
}) {
  const letters = extractManaLetters(description);
  const letter = letters[0] ?? null;
  const bgColor = letter ? MANA_COLORS[letter] : "rgba(0, 0, 0, 0.4)";

  return (
    <button
      className={cn(
        "group/mana flex h-full w-full items-center justify-center transition-all hover:brightness-125",
        className,
      )}
      style={{ backgroundColor: bgColor }}
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
      onMouseDown={(e) => e.preventDefault()}
      title={`Tap: ${description}`}
    >
      <div
        className={cn(
          "flex items-center justify-center rounded-full bg-black/40 shadow-lg transition-transform group-hover/mana:scale-110",
          small ? "h-6 w-6 p-0.5" : "h-8 w-8 p-1",
        )}
      >
        {letter ? (
          <img
            src={manaSymbolUrl(letter)}
            alt={`{${letter}}`}
            className="h-full w-full drop-shadow-md"
            loading="lazy"
          />
        ) : (
          <span className={cn("font-bold text-white", small ? "text-[7px]" : "text-[9px]")}>
            TAP
          </span>
        )}
      </div>
    </button>
  );
});

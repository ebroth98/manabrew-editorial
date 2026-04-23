import { AlertCircle, Check } from "lucide-react";
import { DeckLabelBadge } from "@/components/deck/DeckLabelBadge";
import { DeckCoverImage } from "@/components/deck/deckCover";
import {
  DECK_NAME_SHADOW_CLASS,
  getDeckColorCost,
  getDeckNameColorClass,
  getDeckColors,
} from "@/components/deck/deckDisplay.utils";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { Card, DeckLabel } from "@/types/openmagic";
import type { CardIdentity } from "@/types/server";
import type { DeckCoverSource } from "@/components/deck/deckCover";

interface DeckSelectionCardProps {
  id: string;
  name: string;
  desc?: string;
  color?: string;
  badge?: string | null;
  labels?: DeckLabel[];
  deckList: CardIdentity[];
  cards: Card[];
  cover?: DeckCoverSource;
  isPreset: boolean;
  isSelected: boolean;
  isLegal: boolean;
  validationError?: string;
  onSelect: () => void;
}

function getDeckTypeBreakdown(cards: { types?: string[] }[]): string {
  if (cards.length === 0) return "Empty deck";
  const creatures = cards.filter((card) => card.types?.includes("Creature")).length;
  const lands = cards.filter((card) => card.types?.includes("Land")).length;
  const spells = cards.length - creatures - lands;
  const parts: string[] = [];
  if (creatures > 0) parts.push(`${creatures} creature${creatures === 1 ? "" : "s"}`);
  if (spells > 0) parts.push(`${spells} spell${spells === 1 ? "" : "s"}`);
  if (lands > 0) parts.push(`${lands} land${lands === 1 ? "" : "s"}`);
  return parts.join(" · ");
}

export function DeckSelectionCard({
  id,
  name,
  desc,
  color,
  badge,
  labels,
  deckList,
  cards,
  cover,
  isPreset,
  isSelected,
  isLegal,
  validationError,
  onSelect,
}: DeckSelectionCardProps) {
  const colorCost = getDeckColorCost(cards);
  const titleColorClass = getDeckNameColorClass(cards, isPreset ? color : undefined);
  const breakdown = isPreset ? desc : getDeckTypeBreakdown(cards);
  const fallbackColorLabel = !isPreset && getDeckColors(cards).length === 0;
  const showManaRow = (!isPreset && !!colorCost) || fallbackColorLabel;

  return (
    <button
      key={id}
      type="button"
      onClick={() => {
        if (isLegal) onSelect();
      }}
      disabled={!isLegal}
      title={!isLegal ? validationError : undefined}
      className={cn(
        "relative group rounded-xl border text-left transition-all overflow-hidden",
        "aspect-[4/3] min-h-[172px]",
        isLegal ? "cursor-pointer" : "cursor-not-allowed opacity-50",
        isSelected && isLegal
          ? "border-primary bg-primary/5 ring-1 ring-primary"
          : isLegal
            ? "border-border hover:bg-muted/40 hover:shadow-sm"
            : "border-border",
      )}
    >
      {cover && (
        <>
          <DeckCoverImage cover={cover} alt={name} />
        </>
      )}

      <div className="relative z-10 h-full">
        <div className="absolute right-3 top-3 flex items-center gap-0.5">
          {isSelected && (
            <Check
              className={cn(
                "h-3.5 w-3.5",
                cover ? "text-white drop-shadow-[0_2px_8px_rgba(0,0,0,0.8)]" : "text-primary",
              )}
            />
          )}
          {!isLegal && (
            <AlertCircle
              className={cn(
                "h-3.5 w-3.5",
                cover ? "text-white drop-shadow-[0_2px_8px_rgba(0,0,0,0.8)]" : "text-destructive",
              )}
            />
          )}
        </div>

        <div
          className={cn(
            "absolute inset-x-0 bottom-0 flex flex-col gap-1",
            cover ? "px-3 py-1.5 bg-black/85" : "p-2.5",
          )}
        >
          <div className="flex items-start justify-between gap-2">
            <span
              className={cn(
                "font-semibold text-sm leading-tight line-clamp-2",
                titleColorClass,
                DECK_NAME_SHADOW_CLASS,
              )}
            >
              {name}
            </span>
            {!cover && (
              <div className="flex items-center gap-0.5 shrink-0 mt-0.5">
                {isSelected && <Check className="h-3 w-3 text-primary" />}
                {!isLegal && <AlertCircle className="h-3 w-3 text-destructive" />}
              </div>
            )}
          </div>

          {showManaRow && (
            <div className="flex items-center">
              {!isPreset && colorCost ? (
                <ManaSymbols cost={colorCost} size="sm" />
              ) : fallbackColorLabel ? (
                <span
                  className={cn("text-[10px]", cover ? "text-white/85" : "text-muted-foreground")}
                >
                  Colorless
                </span>
              ) : null}
            </div>
          )}

          <p
            className={cn(
              "text-[11px] leading-tight line-clamp-2",
              cover ? "text-white/85" : "text-muted-foreground",
              DECK_NAME_SHADOW_CLASS,
            )}
          >
            {!isLegal ? validationError : breakdown}
          </p>

          <div className="flex items-center gap-1 flex-wrap">
            <span className={cn("text-[10px]", cover ? "text-white/85" : "text-muted-foreground")}>
              {isPreset ? "Preset deck" : `${deckList.length} cards`}
            </span>
            {labels?.map((label) => (
              <DeckLabelBadge key={label.name} label={label} size="sm" />
            ))}
            {badge && (
              <Badge variant="outline" className="text-[9px] h-4 px-1 ml-auto">
                {badge}
              </Badge>
            )}
          </div>
        </div>
      </div>
    </button>
  );
}

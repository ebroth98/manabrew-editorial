import { Badge } from "@/components/ui/badge";
import { AlertCircle, Check } from "lucide-react";
import { cn } from "@/lib/utils";
import type { Card, DeckLabel } from "@/types/openmagic";
import type { CardIdentity } from "@/types/server";
import { ManaSymbols } from "@/components/game/ManaSymbols";

interface DeckSelectionCardProps {
  id: string;
  name: string;
  desc?: string;
  color?: string;
  badge?: string | null;
  labels?: DeckLabel[];
  deckList: CardIdentity[];
  cards: Card[];
  isPreset: boolean;
  isSelected: boolean;
  isLegal: boolean;
  validationError?: string;
  onSelect: () => void;
}

/** Extract unique WUBRG colors present in a card list, in canonical order. */
function getDeckColors(cards: { color: string }[]): string[] {
  const seen = new Set<string>();
  for (const card of cards) {
    for (const ch of card.color) {
      if ("WUBRG".includes(ch)) seen.add(ch);
    }
  }
  return "WUBRG".split("").filter((c) => seen.has(c));
}

/** Short card-type breakdown string, e.g. "14 creatures · 8 spells · 20 lands". */
function getDeckTypeBreakdown(cards: { types?: string[] }[]): string {
  if (cards.length === 0) return "Empty deck";
  const creatures = cards.filter((c) => c.types?.includes("Creature")).length;
  const lands = cards.filter((c) => c.types?.includes("Land")).length;
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
  isPreset,
  isSelected,
  isLegal,
  validationError,
  onSelect,
}: DeckSelectionCardProps) {
  const colorPips = isPreset ? [] : getDeckColors(cards);
  const breakdown = isPreset ? desc : getDeckTypeBreakdown(cards);

  return (
    <button
      key={id}
      type="button"
      onClick={() => { if (isLegal) onSelect(); }}
      disabled={!isLegal}
      title={!isLegal ? validationError : undefined}
      className={cn(
        "rounded-lg border p-2.5 text-left transition-all",
        isLegal ? "cursor-pointer" : "cursor-not-allowed opacity-50",
        isSelected && isLegal
          ? "border-primary bg-primary/5 ring-1 ring-primary"
          : isLegal
          ? "border-border hover:bg-muted/40 hover:shadow-sm"
          : "border-border"
      )}
    >
      {/* Name row */}
      <div className="flex items-start justify-between gap-1 mb-1.5">
        <span className={cn("font-semibold text-xs leading-tight truncate", isPreset && color)}>
          {name}
        </span>
        <div className="flex items-center gap-0.5 shrink-0 mt-0.5">
          {isSelected && <Check className="h-3 w-3 text-primary" />}
          {!isLegal && <AlertCircle className="h-3 w-3 text-destructive" />}
        </div>
      </div>

      {/* Color identity (user decks only) */}
      {!isPreset && (
        <div className="flex items-center mb-1.5 min-h-[14px]">
          {colorPips.length > 0
            ? <ManaSymbols cost={colorPips.map((c) => `{${c}}`).join("")} size="sm" />
            : <span className="text-[10px] text-muted-foreground">Colorless</span>}
        </div>
      )}

      {/* Type breakdown or description */}
      <p className="text-[10px] text-muted-foreground leading-tight line-clamp-2">
        {!isLegal ? validationError : breakdown}
      </p>

      {/* Footer: card count + labels + badge */}
      <div className="flex items-center gap-1 flex-wrap mt-1.5">
        <span className="text-[10px] text-muted-foreground">
          {deckList.length} cards
        </span>
        {labels?.map((label) => (
          <Badge key={typeof label === 'string' ? label : label.name} variant="outline" className="text-[8px] h-3.5 px-1 text-primary/80 border-primary/30">
            {typeof label === 'string' ? label : label.name}
          </Badge>
        ))}
        {badge && (
          <Badge variant="outline" className="text-[9px] h-4 px-1 ml-auto">
            {badge}
          </Badge>
        )}
      </div>
    </button>
  );
}

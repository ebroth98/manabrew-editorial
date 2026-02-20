import type { Card as CardType } from "@/types/xmage";
import { useCardImage } from "@/hooks/useCardImage";
import { cn } from "@/lib/utils";
import { useState } from "react";
import { CounterDisplay } from "@/components/game/CounterBadge";

interface CardProps {
  card: CardType;
  className?: string;
  isTapped?: boolean;
  onClick?: () => void;
}

function isCreature(card: CardType) {
  return card.types?.some((t) => t.toLowerCase() === "creature");
}

function isLethalDamage(card: CardType) {
  if (!card.damage || !card.toughness) return false;
  const toughness = parseInt(card.toughness, 10);
  return !isNaN(toughness) && card.damage >= toughness;
}

function KeywordChips({ keywords }: { keywords: string[] }) {
  if (!keywords || keywords.length === 0) return null;
  const visible = keywords.slice(0, 2);
  const hidden = keywords.length - visible.length;
  return (
    <div className="absolute top-1 left-1 right-1 flex flex-wrap gap-0.5 z-10">
      {visible.map((kw) => (
        <span
          key={kw}
          className="text-[9px] font-bold uppercase bg-black/60 text-white px-1 py-0.5 rounded leading-none"
        >
          {kw}
        </span>
      ))}
      {hidden > 0 && (
        <span className="text-[9px] font-bold bg-black/60 text-white px-1 py-0.5 rounded leading-none">
          +{hidden}
        </span>
      )}
    </div>
  );
}

export function Card({ card, className, isTapped, onClick }: CardProps) {
  const [hasError, setHasError] = useState(false);
  const { data: scryfallUrl } = useCardImage(card.name, card.imageUrl);
  const imageUrl = card.imageUrl || scryfallUrl;

  const creature = isCreature(card);
  const lethal = isLethalDamage(card);
  const onBattlefield = card.zoneId === "battlefield";

  return (
    <div
      className={cn(
        "relative rounded-lg border bg-card text-card-foreground shadow-sm transition-transform duration-200 ease-in-out hover:scale-105 cursor-pointer overflow-hidden",
        "w-[150px] aspect-[5/7]",
        isTapped && "rotate-90",
        creature && card.summoningSick && onBattlefield && "ring-2 ring-dashed ring-gray-400",
        className
      )}
      title={card.name}
      onClick={onClick}
    >
      {imageUrl && !hasError ? (
        <>
          <img
            src={imageUrl}
            alt={card.name}
            className="absolute inset-0 w-full h-full object-contain rounded-lg"
            onError={() => setHasError(true)}
            loading="lazy"
          />
          {/* Keyword chips — only on battlefield */}
          {onBattlefield && card.keywords && card.keywords.length > 0 && (
            <KeywordChips keywords={card.keywords} />
          )}
          {/* Counter overlay — bottom-left, clear of the P/T box at bottom-right */}
          {card.counters && (
            <CounterDisplay
              counters={card.counters}
              size="sm"
              className="absolute bottom-1 left-1 z-10"
            />
          )}
          {/* P/T overlay — bottom-right, only for creatures */}
          {creature && card.power && card.toughness && (
            <div className="absolute bottom-1 right-1 z-10 flex flex-col items-end gap-0.5">
              <span
                className={cn(
                  "text-[10px] font-bold px-1 py-0.5 rounded leading-none",
                  lethal
                    ? "bg-red-600 text-white"
                    : "bg-black/70 text-white"
                )}
              >
                {card.power}/{card.toughness}
              </span>
              {card.damage != null && card.damage > 0 && (
                <span className="text-[9px] font-bold text-red-400 bg-black/60 px-1 py-0.5 rounded leading-none">
                  ⚔{card.damage}
                </span>
              )}
            </div>
          )}
        </>
      ) : (
        <div className="absolute inset-0 p-2 flex flex-col justify-between">
          <div className="flex justify-between items-start gap-1">
            <span className="font-bold text-xs leading-tight line-clamp-2">{card.name}</span>
            <span className="text-xs font-mono shrink-0">{card.manaCost}</span>
          </div>
          <div className="flex-1 flex items-center justify-center px-1">
            <span className="text-xs text-muted-foreground text-center line-clamp-5">
              {card.text}
            </span>
          </div>
          {/* Counters row in text fallback */}
          {card.counters && (
            <CounterDisplay counters={card.counters} size="sm" className="mb-0.5" />
          )}
          <div className="flex justify-between items-end">
            <span className="text-xs text-muted-foreground truncate">
              {card.types?.join(" ")}
            </span>
            {creature && card.power && card.toughness && (
              <span className={cn("font-bold text-sm shrink-0", lethal && "text-red-500")}>
                {card.power}/{card.toughness}
                {card.damage != null && card.damage > 0 && (
                  <span className="text-xs text-red-400 ml-0.5">⚔{card.damage}</span>
                )}
              </span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

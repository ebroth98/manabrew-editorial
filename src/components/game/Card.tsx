import type { Card as CardType } from "@/types/xmage";
import { useCardImage } from "@/hooks/useCardImage";
import { cn } from "@/lib/utils";
import { useState } from "react";
import { CounterDisplay } from "@/components/game/CounterBadge";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { KeywordChips } from "@/components/game/CardKeywords";
import { isCreature, isLethalDamage } from "./game.utils";

interface CardProps {
  card: CardType;
  className?: string;
  isTapped?: boolean;
  onClick?: () => void;
  isHovered?: boolean;
  onFlip?: () => void;
  showBackFace?: boolean;
}

export function Card({
  card,
  className,
  isTapped,
  onClick,
  isHovered,
  onFlip,
  showBackFace,
}: CardProps) {
  const [hasError, setHasError] = useState(false);
  const { data: scryfallUrl } = useCardImage(
    card.name,
    card.imageUrl,
    card.isToken,
    card.color,
    card.setCode,
  );
  const imageUrl = card.imageUrl || scryfallUrl;

  const creature = isCreature(card);
  const lethal = isLethalDamage(card);
  const onBattlefield = card.zoneId === "battlefield";

  return (
    <div
      className={cn(
        "relative rounded-lg border bg-card text-card-foreground shadow-sm transition-transform duration-200 ease-in-out hover:scale-105 cursor-pointer group overflow-hidden",
        "w-[150px] aspect-[5/7]",
        isTapped && "rotate-90",
        creature &&
          card.summoningSick &&
          onBattlefield &&
          "ring-2 ring-dashed ring-gray-400",
        className,
      )}
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
          {/* Exerted indicator */}
          {card.exerted && (
            <div className="absolute top-0 left-0 right-0 flex justify-center z-20 pointer-events-none">
              <span className="text-[8px] font-bold px-1.5 py-0.5 rounded-b leading-none bg-orange-500/90 text-white">
                EXERTED
              </span>
            </div>
          )}
          {/* Token / Transformed indicator — top-center banner */}
          {!card.exerted && (card.isToken || card.isTransformed) && (
            <div className="absolute top-0 left-0 right-0 flex justify-center z-20 pointer-events-none">
              <span
                className={cn(
                  "text-[8px] font-bold px-1.5 py-0.5 rounded-b leading-none",
                  card.isTransformed
                    ? "bg-purple-500/90 text-white"
                    : "bg-amber-400/90 text-amber-900",
                )}
              >
                {card.isTransformed ? "TRANSFORMED" : "TOKEN"}
              </span>
            </div>
          )}
          {/* Keyword chips — all zones */}
          {card.keywords && card.keywords.length > 0 && (
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
                  lethal ? "bg-red-600 text-white" : "bg-black/70 text-white",
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
            <span className="font-bold text-xs leading-tight line-clamp-2">
              {card.name}
            </span>
            <div className="flex flex-col items-end gap-0.5 shrink-0">
              {(card.isToken || card.isTransformed) && (
                <span
                  className={cn(
                    "text-[8px] font-bold px-1 py-0.5 rounded leading-none",
                    card.isTransformed
                      ? "bg-purple-500/90 text-white"
                      : "bg-amber-400/90 text-amber-900",
                  )}
                >
                  {card.isTransformed ? "TRANSFORMED" : "TOKEN"}
                </span>
              )}
              {card.effectiveManaCost ? (
                <div className="flex flex-col items-end">
                  <span className="line-through opacity-50">
                    <ManaSymbols cost={card.manaCost} size="sm" />
                  </span>
                  <span className="bg-green-500/20 rounded px-0.5">
                    <ManaSymbols cost={card.effectiveManaCost} size="sm" />
                  </span>
                </div>
              ) : (
                <ManaSymbols cost={card.manaCost} size="sm" />
              )}
            </div>
          </div>
          <div className="flex-1 flex items-center justify-center px-1">
            <span className="text-xs text-muted-foreground text-center line-clamp-5">
              {card.text}
            </span>
          </div>
          {/* Counters row in text fallback */}
          {card.counters && (
            <CounterDisplay
              counters={card.counters}
              size="sm"
              className="mb-0.5"
            />
          )}
          <div className="flex justify-between items-end">
            <span className="text-xs text-muted-foreground truncate">
              {card.types?.join(" ")}
            </span>
            {creature && card.power && card.toughness && (
              <span
                className={cn(
                  "font-bold text-sm shrink-0",
                  lethal && "text-red-500",
                )}
              >
                {card.power}/{card.toughness}
                {card.damage != null && card.damage > 0 && (
                  <span className="text-xs text-red-400 ml-0.5">
                    ⚔{card.damage}
                  </span>
                )}
              </span>
            )}
          </div>
        </div>
      )}

      {/* Flip button for double-faced cards - appears on hover */}
      {card.isDoubleFaced && (
        <div
          className={cn(
            "absolute left-1/2 -translate-x-1/2 bottom-0 z-50",
            isHovered ? "flex" : "hidden group-hover:flex",
          )}
        >
          <button
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              onFlip?.();
            }}
            className="bg-black/90 hover:bg-black text-white px-2.5 py-1 rounded shadow-lg border border-white/20 transition-colors pointer-events-auto flex items-center gap-1.5 whitespace-nowrap text-xs backdrop-blur-sm"
            title={showBackFace ? "Show Front Face" : "Show Back Face"}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M8 3H5a2 2 0 0 0-2 2v3" />
              <path d="M16 3h3a2 2 0 0 1 2 2v3" />
              <path d="M12 20v-18" />
              <path d="M8 21H5a2 2 0 0 1-2-2v-3" />
              <path d="M16 21h3a2 2 0 0 0 2-2v-3" />
            </svg>
          </button>
        </div>
      )}
    </div>
  );
}

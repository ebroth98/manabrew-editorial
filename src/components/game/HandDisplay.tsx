import { useState } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import type { HandDisplayProps } from "./game.types";
import { HAND_CARD, ZONE_LABEL } from "./game.styles";

export function HandDisplay({
  cards,
  onHoverCard,
  onStartDrag,
  onFlipCard,
  showBackFace,
}: HandDisplayProps) {
  const [hoveredCardId, setHoveredCardId] = useState<string | null>(null);

  return (
    <div className="flex flex-col gap-1 shrink-0">
      <span className={ZONE_LABEL}>
        Hand ({cards.length})
      </span>
      <div className="overflow-x-auto">
        <div className="flex gap-2 pb-2 px-1 min-h-[120px] items-end">
          {cards.map((card) => (
            <div
              key={card.id}
              className={cn(
                "relative group shrink-0",
                card.isPlayable && "cursor-grab",
              )}
              onMouseDown={
                card.isPlayable
                  ? (e) => { e.preventDefault(); onStartDrag?.(card, e); }
                  : undefined
              }
              onMouseEnter={(e) => {
                setHoveredCardId(card.id);
                onHoverCard?.(card, e);
              }}
              onMouseLeave={() => {
                setHoveredCardId(null);
                onHoverCard?.(null);
              }}
            >
              <Card
                card={card}
                className={cn(
                  HAND_CARD, "transition-transform group-hover:-translate-y-3",
                  !card.isPlayable && "opacity-60 grayscale",
                )}
                isHovered={hoveredCardId === card.id}
                onFlip={onFlipCard}
                showBackFace={showBackFace}
              />
              {card.isPlayable && (
                <div
                  className="absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 bg-primary/20 border-2 border-primary transition-opacity pointer-events-none"
                  title={`Play ${card.name}`}
                />
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

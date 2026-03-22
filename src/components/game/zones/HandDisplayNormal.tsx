import { useState } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { useHandScale } from "@/hooks/useHandScale";
import type { HandDisplayProps } from "../game.types";
import { HAND_CARD_BASES, ZONE_LABEL } from "../game.styles";



export function HandDisplayNormal({
  cards,
  onHoverCard,
  onStartDrag,
  onFlipCard,
  showBackFace,
  draggingCardId,
}: HandDisplayProps) {
  const handSize = usePreferencesStore((s) => s.handSize);
  const vScale = useHandScale();
  const base = HAND_CARD_BASES[handSize];
  const cardW = Math.round(base.cardW * vScale);
  const cardH = Math.round(base.cardH * vScale);
  const containerH = Math.round(base.containerH * vScale);
  const [hoveredCardId, setHoveredCardId] = useState<string | null>(null);

  return (
    <div className="flex flex-col gap-1 shrink-0">
      <span className={ZONE_LABEL}>Hand ({cards.length})</span>
      <div className="overflow-x-auto">
        <div className="flex gap-2 pt-4 pb-2 px-1 items-end" style={{ minHeight: containerH - 8 }}>
          {cards.map((card) => (
            <div
              key={card.id}
              className={cn(
                "relative group shrink-0 transition-[transform,z-index] duration-250 ease-[cubic-bezier(0.23,0.63,0.32,1)]",
                hoveredCardId === card.id && "-translate-y-3 z-30",
                card.isPlayable && "cursor-grab",
                card.id === draggingCardId && "opacity-0",
              )}
              onMouseDown={
                card.isPlayable
                  ? (e) => {
                      e.preventDefault();
                      onStartDrag?.(card, e);
                    }
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
                  "transition-transform duration-250 ease-[cubic-bezier(0.23,0.63,0.32,1)] hover:scale-100",
                  card.isPlayable && cn("playable-card", hoveredCardId === card.id && "is-hovered"),
                )}
                style={{ width: cardW, height: cardH }}
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

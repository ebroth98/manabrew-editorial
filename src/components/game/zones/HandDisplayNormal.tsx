import { useState, useRef, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { useHandScale } from "@/hooks/useHandScale";
import type { HandDisplayProps } from "../game.types";
import { HAND_CARD_BASES, ZONE_LABEL } from "../game.styles";

const TUG_LIMIT = 100;

export function HandDisplayNormal({
  cards,
  onHoverCard,
  onStartDrag,
  onClickCard,
  onFlipCard,
  showBackFace,
  draggingCardId,
  castingCardId,
}: HandDisplayProps) {
  const handSize = usePreferencesStore((s) => s.handSize);
  const vScale = useHandScale();
  const base = HAND_CARD_BASES[handSize];
  const cardW = Math.round(base.cardW * vScale);
  const cardH = Math.round(base.cardH * vScale);
  const containerH = Math.round(base.containerH * vScale);

  const [rejectedId, setRejectedId] = useState<string | null>(null);
  const rejectedTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const rejectCard = useCallback((id: string) => {
    clearTimeout(rejectedTimer.current);
    setRejectedId(id);
    rejectedTimer.current = setTimeout(() => setRejectedId(null), 400);
  }, []);

  const [tugId, setTugId] = useState<string | null>(null);
  const [tugOffset, setTugOffset] = useState({ x: 0, y: 0 });

  const startTug = useCallback((cardId: string, startX: number, startY: number) => {
    setTugId(cardId);
    setTugOffset({ x: 0, y: 0 });

    const onMove = (me: MouseEvent) => {
      const dx = me.clientX - startX;
      const dy = me.clientY - startY;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist > TUG_LIMIT) {
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
        setTugId(null);
        setTugOffset({ x: 0, y: 0 });
        rejectCard(cardId);
      } else {
        setTugOffset({ x: dx, y: dy });
      }
    };

    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      setTugId(null);
      setTugOffset({ x: 0, y: 0 });
      rejectCard(cardId);
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }, [rejectCard]);

  const [hoveredCardId, setHoveredCardId] = useState<string | null>(null);

  return (
    <div className="flex flex-col gap-1 shrink-0">
      <span className={ZONE_LABEL}>Hand ({cards.length})</span>
      <div className="overflow-x-auto">
        <div className="flex gap-2 pt-4 pb-2 px-1 items-end" style={{ minHeight: containerH - 8 }}>
          {cards.map((card) => {
            const isCasting = castingCardId != null && card.id === castingCardId;
            const isTugging = tugId === card.id;
            return (
              <div
                key={card.id}
                className={cn(
                  "relative group shrink-0",
                  !isTugging && "transition-[transform,z-index] duration-250 ease-[cubic-bezier(0.23,0.63,0.32,1)]",
                  hoveredCardId === card.id && !isTugging && "-translate-y-3 z-30",
                  card.isPlayable && "cursor-grab",
                  (card.id === draggingCardId || isCasting) && "opacity-0",
                )}
                style={isTugging ? { transform: `translate(${tugOffset.x}px, ${tugOffset.y}px)`, zIndex: 100 } : undefined}
                onMouseDown={(e) => {
                  e.preventDefault();
                  if (card.isPlayable && onStartDrag) {
                    onStartDrag?.(card, e);
                  } else if (card.isPlayable) {
                    onClickCard?.(card, e);
                  } else {
                    startTug(card.id, e.clientX, e.clientY);
                  }
                }}
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
                    rejectedId === card.id && "animate-reject-flash",
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
            );
          })}
        </div>
      </div>
    </div>
  );
}

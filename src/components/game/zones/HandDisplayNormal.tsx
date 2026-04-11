import { useState, useRef, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { useHandScale } from "@/hooks/useHandScale";
import type { HandDisplayProps } from "../game.types";
import { HAND_CARD_BASES, ZONE_LABEL } from "../game.styles";
import { HandCardActions } from "./HandCardActions";

const TUG_LIMIT = 100;
const HOVER_SCALE = 1.8;

export function HandDisplayNormal({
  cards,
  onHoverCard,
  onStartDrag,
  onClickCard,
  onFlipCard,
  showBackFace,
  draggingCardId,
  castingCardId,
  getActions,
  onSelectAction,
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
  const containerRef = useRef<HTMLDivElement>(null);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  return (
    <div className="flex flex-col gap-1 shrink-0" ref={containerRef}>
      <span className={ZONE_LABEL}>Hand ({cards.length})</span>
      <div className="overflow-x-auto">
        <div className="flex gap-2 pt-4 pb-2 px-1 items-end" style={{ minHeight: containerH - 8 }}>
          {cards.map((card) => {
            const isCasting = castingCardId != null && card.id === castingCardId;
            const isTugging = tugId === card.id;
            const isHov = hoveredCardId === card.id;
            const actions = isHov && getActions ? getActions(card) : [];

            const scale = isHov ? HOVER_SCALE : 1;

            return (
              <div
                key={card.id}
                className={cn(
                  "relative group shrink-0",
                  !isTugging && "transition-[transform,z-index] duration-250 ease-[cubic-bezier(0.23,0.63,0.32,1)]",
                  isHov && !isTugging && "-translate-y-3 z-30",
                  card.isPlayable && "cursor-grab",
                  (card.id === draggingCardId || isCasting) && "opacity-0",
                )}
                style={{
                  width: cardW,
                  height: cardH,
                  ...(isTugging ? { transform: `translate(${tugOffset.x}px, ${tugOffset.y}px)`, zIndex: 100 } : {}),
                }}
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
                  clearTimeout(hideTimerRef.current);
                  setHoveredCardId(card.id);
                  const el = e.currentTarget as HTMLElement;
                  const rect = el.getBoundingClientRect();
                  const finalTop = rect.top - 12 - (cardH * HOVER_SCALE - cardH);

                  onHoverCard?.(card, e, {
                    useAnchor: true,
                    placement: "top-center",
                    anchorOverride: {
                      left: rect.left,
                      right: rect.right,
                      top: finalTop,
                      bottom: finalTop + cardH * HOVER_SCALE,
                      width: cardW * HOVER_SCALE,
                      height: cardH * HOVER_SCALE,
                      x: rect.left,
                      y: finalTop,
                      toJSON: () => ({})
                    } as DOMRect,
                  });
                }}
                onMouseLeave={() => {
                  hideTimerRef.current = setTimeout(() => {
                    setHoveredCardId(null);
                    onHoverCard?.(null);
                  }, 150);
                }}
              >
                <div className="w-full h-full relative" style={{
                  transform: isHov ? `scale(${HOVER_SCALE})` : "scale(1)",
                  transformOrigin: "bottom center",
                  transition: "transform 250ms cubic-bezier(0.23, 0.63, 0.32, 1)",
                }}>
                  <Card
                    card={card}
                    className={cn(
                      "w-full h-full",
                      card.isPlayable && cn("playable-card", isHov && "is-hovered"),
                      rejectedId === card.id && "animate-reject-flash",
                    )}
                    isHovered={isHov}
                    onFlip={onFlipCard}
                    showBackFace={showBackFace}
                    resolution="large"
                  />
                  {card.isPlayable && (
                    <div
                      className="absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"
                      style={{
                        backgroundColor: "var(--playable-glow-color, rgba(251, 146, 60, 0.3))",
                        border: "2px solid var(--playable-ring-color-strong, rgba(251, 146, 60, 1))"
                      }}
                      title={`Play ${card.name}`}
                    />
                  )}

                  {isHov && actions.length > 0 && onSelectAction && (
                    <div style={{
                      position: "absolute",
                      top: 0,
                      left: "100%",
                      transform: `scale(${1 / scale})`,
                      transformOrigin: "top left",
                    }}>
                      {/* Curved invisible bridge to maintain hover without blocking cards below */}
                      <div
                        style={{
                          position: "absolute",
                          top: 0,
                          left: -cardW * scale,
                          width: cardW * scale + 24 + 220,
                          height: cardH * scale,
                          backgroundColor: "transparent",
                          borderBottomRightRadius: "100%",
                          zIndex: -1,
                        }}
                      />
                      
                      <div style={{ paddingLeft: 24 }}>
                        <HandCardActions 
                          actions={actions} 
                          onSelectAction={onSelectAction} 
                        />
                      </div>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

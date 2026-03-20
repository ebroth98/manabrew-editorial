import { useState, useMemo, useRef } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import type { HandDisplayProps } from "../game.types";
import { HAND_CARD } from "../game.styles";



const CARD_W = 80;
const ARC_RADIUS = 900;
const MAX_ARC_DEG = 30;
const HOVER_LIFT = 28;
const HOVER_SCALE = 1.22;
const NEIGHBOR_PUSH = 30;
const MAX_SPREAD = 56;
const MIN_SPREAD = 24;
const SPREAD_WIDTH = 560;

function computeLayout(count: number) {
  if (count === 0) return [];
  if (count === 1)
    return [{ x: 0, drop: 0, rot: 0 }];

  const spread = Math.max(
    MIN_SPREAD,
    Math.min(MAX_SPREAD, Math.floor((SPREAD_WIDTH - CARD_W) / (count - 1))),
  );
  const totalWidth = (count - 1) * spread;
  const arcDeg = Math.min(MAX_ARC_DEG, count * 2.5);

  return Array.from({ length: count }, (_, i) => {
    const t = count === 1 ? 0 : (i / (count - 1)) * 2 - 1;
    const x = -totalWidth / 2 + i * spread;
    const rot = t * (arcDeg / 2);
    const drop = (1 - Math.cos((t * Math.PI) / 2)) * (ARC_RADIUS * 0.015);
    return { x, drop, rot };
  });
}

export function HandDisplayCool({
  cards,
  onHoverCard,
  onStartDrag,
  onFlipCard,
  showBackFace,
  draggingCardId,
}: HandDisplayProps) {
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const positions = useMemo(() => computeLayout(cards.length), [cards.length]);
  const containerRef = useRef<HTMLDivElement>(null);
  const hoveredIdRef = useRef<string | null>(null);

  const hovIdx = hoveredId ? cards.findIndex((c) => c.id === hoveredId) : -1;

  const handleMouseMove = (e: React.MouseEvent) => {
    const container = containerRef.current;
    if (!container || cards.length === 0) return;

    const rect = container.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const mouseX = e.clientX - centerX;

    let closest = 0;
    let closestDist = Infinity;
    for (let i = 0; i < positions.length; i++) {
      const dist = Math.abs(mouseX - positions[i].x);
      if (dist < closestDist) {
        closestDist = dist;
        closest = i;
      }
    }

    if (closestDist > CARD_W) {
      if (hoveredIdRef.current !== null) {
        hoveredIdRef.current = null;
        setHoveredId(null);
        onHoverCard?.(null);
      }
      return;
    }

    const card = cards[closest];
    if (card.id !== hoveredIdRef.current) {
      hoveredIdRef.current = card.id;
      setHoveredId(card.id);
      onHoverCard?.(card, e);
    }
  };

  const handleMouseLeave = () => {
    hoveredIdRef.current = null;
    setHoveredId(null);
    onHoverCard?.(null);
  };

  const containerWidth = Math.max(
    CARD_W + 40,
    (positions[positions.length - 1]?.x ?? 0) - (positions[0]?.x ?? 0) + CARD_W + 80,
  );

  return (
    <div className="-mb-4 flex flex-col items-center gap-1 shrink-0">
      <div
        ref={containerRef}
        className="relative"
        style={{ height: 140, width: containerWidth }}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
      >
        {cards.map((card, idx) => {
          const pos = positions[idx];
          const isHov = hoveredId === card.id;

          let pushX = 0;
          if (hovIdx >= 0 && idx !== hovIdx) {
            const dist = Math.abs(idx - hovIdx);
            const sign = idx < hovIdx ? -1 : 1;
            pushX = sign * Math.max(0, NEIGHBOR_PUSH - dist * 6);
          }

          const tx = pos.x + pushX;
          const translateY = isHov ? -HOVER_LIFT : pos.drop;
          const rot = isHov ? 0 : pos.rot;
          const scale = isHov ? HOVER_SCALE : 1;
          const z = isHov ? 100 : idx + 1;

          return (
            <div
              key={card.id}
              className={cn(
                "absolute will-change-transform isolate pointer-events-none",
                card.isPlayable && "cursor-grab",
                card.id === draggingCardId && "opacity-0",
              )}
              style={{
                left: "50%",
                bottom: 0,
                transform: `translateX(${tx - CARD_W / 2}px) translateY(${translateY}px) rotate(${rot}deg) scale(${scale})`,
                transformOrigin: "center bottom",
                transition: "transform 280ms cubic-bezier(0.34, 1.56, 0.64, 1)",
                zIndex: z,
              }}
            >
              <div
                className="pointer-events-auto"
                onMouseDown={
                  card.isPlayable
                    ? (e) => {
                        e.preventDefault();
                        onStartDrag?.(card, e);
                      }
                    : undefined
                }
              >
                <Card
                  card={card}
                  className={cn(
                    HAND_CARD,
                    "shadow-md !bg-card",
                    isHov && "shadow-xl shadow-black/40",
                    card.isPlayable && cn("playable-card", isHov && "is-hovered"),
                  )}
                  isHovered={isHov}
                  onFlip={onFlipCard}
                  showBackFace={showBackFace}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

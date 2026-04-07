import { useState, useMemo, useRef, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { useHandScale } from "@/hooks/useHandScale";
import type { HandDisplayProps } from "../game.types";
import { HAND_CARD_BASES } from "../game.styles";

const ARC_RADIUS = 900;
const MAX_ARC_DEG = 30;
const HOVER_SCALE = 1.22;

/** Base layout params at 1920px reference, keyed by size preference. */
const SIZE_PARAMS = {
  small:  { hoverLift: 28, neighborPush: 30, maxSpread: 56, minSpread: 24, spreadWidth: 560 },
  medium: { hoverLift: 46, neighborPush: 48, maxSpread: 90, minSpread: 38, spreadWidth: 900 },
  large:  { hoverLift: 60, neighborPush: 62, maxSpread: 118, minSpread: 50, spreadWidth: 1180 },
} as const;

function computeLayout(
  count: number,
  cardW: number,
  maxSpread: number,
  minSpread: number,
  spreadWidth: number,
) {
  if (count === 0) return [];
  if (count === 1) return [{ x: 0, drop: 0, rot: 0 }];

  const spread = Math.max(
    minSpread,
    Math.min(maxSpread, Math.floor((spreadWidth - cardW) / (count - 1))),
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
  onClickCard,
  onFlipCard,
  showBackFace,
  draggingCardId,
  castingCardId,
}: HandDisplayProps) {
  const handSize = usePreferencesStore((s) => s.handSize);
  const vScale = useHandScale();
  const base = HAND_CARD_BASES[handSize];
  const params = SIZE_PARAMS[handSize];

  // Scaled values
  const cardW = Math.round(base.cardW * vScale);
  const cardH = Math.round(base.cardH * vScale);
  const containerH = Math.round(base.containerH * vScale);
  const hoverLift = Math.round(params.hoverLift * vScale);
  const neighborPush = Math.round(params.neighborPush * vScale);
  const maxSpread = Math.round(params.maxSpread * vScale);
  const minSpread = Math.round(params.minSpread * vScale);
  const spreadWidth = Math.round(params.spreadWidth * vScale);

  const [rejectedId, setRejectedId] = useState<string | null>(null);
  const rejectedTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const rejectCard = useCallback((id: string) => {
    clearTimeout(rejectedTimer.current);
    setRejectedId(id);
    rejectedTimer.current = setTimeout(() => setRejectedId(null), 400);
  }, []);

  // "Tug" state — non-playable cards can be dragged a few px before snapping back
  const TUG_LIMIT = 100;
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
        // Hit the limit — snap back and flash
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

  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const positions = useMemo(
    () => computeLayout(cards.length, cardW, maxSpread, minSpread, spreadWidth),
    [cards.length, cardW, maxSpread, minSpread, spreadWidth],
  );
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

    if (closestDist > cardW) {
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
    cardW + 40,
    (positions[positions.length - 1]?.x ?? 0) - (positions[0]?.x ?? 0) + cardW + 80,
  );

  return (
    <div className="-mb-4 flex flex-col items-center gap-1 shrink-0">
      <div
        ref={containerRef}
        className="relative"
        style={{ height: containerH, width: containerWidth }}
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
            pushX = sign * Math.max(0, neighborPush - dist * 6);
          }

          const isCasting = castingCardId != null && card.id === castingCardId;
          const isTugging = tugId === card.id;
          const tx = pos.x + pushX + (isTugging ? tugOffset.x : 0);
          const translateY = (isHov ? -hoverLift : pos.drop) + (isTugging ? tugOffset.y : 0);
          const rot = isHov ? 0 : pos.rot;
          const scale = isHov ? HOVER_SCALE : 1;
          const z = isTugging ? 100 : isHov ? 100 : idx + 1;

          return (
            <div
              key={card.id}
              className={cn(
                "absolute will-change-transform isolate pointer-events-none",
                card.isPlayable && "cursor-grab",
                (card.id === draggingCardId || isCasting) && "opacity-0",
              )}
              style={{
                left: "50%",
                bottom: 0,
                transform: `translateX(${tx - cardW / 2}px) translateY(${translateY}px) rotate(${rot}deg) scale(${scale})`,
                transformOrigin: "center bottom",
                transition: isTugging ? "none" : "transform 280ms cubic-bezier(0.34, 1.56, 0.64, 1)",
                zIndex: z,
              }}
            >
              <div
                className="pointer-events-auto"
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
              >
                <Card
                  card={card}
                  className={cn(
                    "shadow-md !bg-card",
                    isHov && "shadow-xl shadow-black/40",
                    card.isPlayable && cn("playable-card", isHov && "is-hovered"),
                    rejectedId === card.id && "animate-reject-flash",
                  )}
                  style={{ width: cardW, height: cardH }}
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

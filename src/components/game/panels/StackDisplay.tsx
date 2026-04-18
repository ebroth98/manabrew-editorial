import { useEffect, useRef, useState } from "react";
import { Card } from "@/components/game/Card";
import { cn } from "@/lib/utils";
import type { Card as XMageCard, StackObject } from "@/types/openmagic";
import { useStackUIStore } from "@/stores/useStackUIStore";

interface StackDisplayProps {
  stack: StackObject[];
  resolveStackCard: (stackItem: StackObject) => XMageCard;
  onOpenStack: () => void;
  flashCard?: XMageCard | null;
  flashToken?: string | null;
  showPreStackFlash?: boolean;
  /** Card currently being cast (waiting for targets / mana payment). */
  castingCard?: XMageCard | null;
  /**
   * When the right action panel is open it covers the default stack
   * position — callers pass `false` so the stack shifts leftward.
   */
  rightPanelCollapsed?: boolean;
}

// Stack UI tuning (single source of truth for size/placement)
const STACK_CARD_ASPECT = 7 / 5; // MTG card ratio: 5:7 (w:h)
// Right inset when the action panel is open (panel is w-72 = 288px + its
// own right-1.5 gap ≈ 6px). Add a small breathing gap so the stack cards
// don't touch the panel edge.
const STACK_RIGHT_WHEN_PANEL_OPEN = 288 + 6 + 8;
const STACK_RIGHT_WHEN_PANEL_COLLAPSED = 10;
const STACK_UI = {
  direction: "left" as "left" | "right",
  cardWidth: 220,
  offsetX: 15,
  offsetY: 2,
  centerOffsetY: -60,
  hoverPushX: 60,
} as const;

export function StackDisplay({
  stack,
  resolveStackCard,
  onOpenStack,
  flashCard,
  flashToken,
  showPreStackFlash = true,
  castingCard,
  rightPanelCollapsed = true,
}: StackDisplayProps) {
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const setHoveredStackObjectId = useStackUIStore((s) => s.setHoveredStackObjectId);
  const prevLayoutRef = useRef<Record<string, { left: number; top: number }>>({});
  const prevStackIdsRef = useRef<Set<string>>(new Set(stack.map((obj) => obj.id)));
  const enteringIds = new Set(
    stack
      .filter((obj) => !prevStackIdsRef.current.has(obj.id))
      .map((obj) => obj.id),
  );

  const hoveredIndex = hoveredId ? stack.findIndex((obj) => obj.id === hoveredId) : -1;
  const flashStackIndex = flashCard
    ? stack.findIndex((obj) => obj.sourceId === flashCard.id)
    : -1;
  const directionSign = STACK_UI.direction === "right" ? 1 : -1;
  const cardHeight = Math.round(STACK_UI.cardWidth * STACK_CARD_ASPECT);
  const totalItems = stack.length + (castingCard ? 1 : 0);
  const spanX = Math.max(0, totalItems - 1) * STACK_UI.offsetX;
  const pileHeight = cardHeight + Math.max(0, totalItems - 1) * Math.abs(STACK_UI.offsetY);

  const baseLefts = stack.map((_, idx) => idx * STACK_UI.offsetX * directionSign);
  const lefts =
    hoveredIndex < 0
      ? baseLefts
      : stack.map((_, idx) => {
          if (idx === hoveredIndex) return baseLefts[idx];
          const pushSign = (idx < hoveredIndex ? -1 : 1) * directionSign;
          return baseLefts[idx] + pushSign * STACK_UI.hoverPushX;
        });
  // Stable bounds by direction so hover transitions don't snap/rebase.
  const fixedMinLeft =
    STACK_UI.direction === "right"
      ? -STACK_UI.hoverPushX
      : -spanX - STACK_UI.hoverPushX;
  const fixedMaxLeft =
    STACK_UI.direction === "right"
      ? spanX + STACK_UI.hoverPushX
      : STACK_UI.hoverPushX;
  const xShift = -fixedMinLeft;
  const pileWidth = fixedMaxLeft - fixedMinLeft + STACK_UI.cardWidth;
  const top = `calc(50% - ${pileHeight / 2}px + ${STACK_UI.centerOffsetY}px)`;

  useEffect(() => {
    const nextLayout: Record<string, { left: number; top: number }> = {};
    stack.forEach((obj, idx) => {
      nextLayout[obj.id] = {
        left: lefts[idx] + xShift,
        top: idx * STACK_UI.offsetY,
      };
    });
    prevLayoutRef.current = nextLayout;
  }, [stack, lefts, xShift]);

  useEffect(() => {
    prevStackIdsRef.current = new Set(stack.map((obj) => obj.id));
  }, [stack]);

  useEffect(() => {
    return () => setHoveredStackObjectId(null);
  }, [setHoveredStackObjectId]);

  if (stack.length === 0 && !flashCard && !castingCard) return null;

  const rightInset = rightPanelCollapsed
    ? STACK_RIGHT_WHEN_PANEL_COLLAPSED
    : STACK_RIGHT_WHEN_PANEL_OPEN;

  return (
    <div
      data-stack-panel
      className={cn(
        "pointer-events-auto absolute z-40 transition-[right] duration-200",
      )}
      style={{ top, right: `${rightInset}px` }}
    >
      <div
        className="relative"
        style={{ height: `${pileHeight}px`, width: `${pileWidth}px` }}
        onMouseLeave={() => {
          setHoveredId(null);
          setHoveredStackObjectId(null);
        }}
      >
        {stack.map((obj, idx) => {
          const card = resolveStackCard(obj);
          const isHovered = hoveredId === obj.id;
          const isTopOfStack = idx === stack.length - 1;
          const isFlashedStackCard = flashStackIndex === idx;
          const targetLeft = lefts[idx] + xShift;
          const targetTop = idx * STACK_UI.offsetY;
          const prev = prevLayoutRef.current[obj.id];
          const hasPositionChange =
            !prev || prev.left !== targetLeft || prev.top !== targetTop;
          const zIndex =
            hoveredIndex < 0
              ? idx + 1
              : 200 - Math.abs(idx - hoveredIndex) * 10 + (isHovered ? 5 : 0);
          return (
            <div
              key={obj.id}
              data-stack-object-id={obj.id}
              data-card-id={obj.sourceId}
              className={cn(
                "absolute left-0 will-change-transform",
                hasPositionChange
                  ? "transition-[left,top,transform] duration-560 ease-[cubic-bezier(0.23,0.63,0.32,1)]"
                  : "transition-transform duration-560 ease-[cubic-bezier(0.23,0.63,0.32,1)]",
                isHovered && "-translate-y-2 scale-105",
              )}
              style={{
                left: `${targetLeft}px`,
                top: `${targetTop}px`,
                zIndex,
                width: `${STACK_UI.cardWidth}px`,
                height: `${cardHeight}px`,
              }}
              onMouseEnter={() => {
                setHoveredId(obj.id);
                setHoveredStackObjectId(obj.id);
              }}
              onClick={onOpenStack}
            >
              <Card
                card={card}
                className={cn(
                  "w-full h-full shadow-lg cursor-pointer",
                  isFlashedStackCard && "animate-card-stack-flash-in",
                  enteringIds.has(obj.id) &&
                    !isFlashedStackCard &&
                    "animate-card-stack-enter",
                  isTopOfStack && "playable-card",
                )}
              />
            </div>
          );
        })}

        {castingCard && (
          <div
            data-casting-card={castingCard.id}
            className="absolute left-0"
            style={{
              zIndex: stack.length + 2,
              left: `${(stack.length) * STACK_UI.offsetX * directionSign + xShift}px`,
              top: `${stack.length * STACK_UI.offsetY}px`,
              width: `${STACK_UI.cardWidth}px`,
              height: `${cardHeight}px`,
            }}
          >
            <Card
              card={castingCard}
              className="w-full h-full shadow-lg casting-card"
            />
          </div>
        )}

        {flashCard && flashStackIndex < 0 && showPreStackFlash && (
          <div
            key={flashToken ?? flashCard.id}
            className="absolute left-0 top-0 pointer-events-none animate-card-flash"
            style={{
              zIndex: 200,
              left: "0px",
              top: "0px",
              width: `${STACK_UI.cardWidth}px`,
              height: `${cardHeight}px`,
            }}
          >
            <Card card={flashCard} className="w-full h-full shadow-2xl" />
          </div>
        )}
      </div>
    </div>
  );
}

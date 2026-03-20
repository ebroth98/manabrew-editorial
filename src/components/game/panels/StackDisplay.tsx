import { useEffect, useRef, useState } from "react";
import { Card } from "@/components/game/Card";
import { cn } from "@/lib/utils";
import type { Card as XMageCard, StackObject } from "@/types/openmagic";

interface StackDisplayProps {
  stack: StackObject[];
  resolveStackCard: (stackItem: StackObject) => XMageCard;
  onOpenStack: () => void;
  flashCard?: XMageCard | null;
  flashToken?: string | null;
  showPreStackFlash?: boolean;
}

// Stack UI tuning (single source of truth for size/placement)
const STACK_CARD_ASPECT = 7 / 5; // MTG card ratio: 5:7 (w:h)
const STACK_UI = {
  positionClass: "absolute right-[10px] z-40",
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
}: StackDisplayProps) {
  const [hoveredId, setHoveredId] = useState<string | null>(null);
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
  const spanX = Math.max(0, stack.length - 1) * STACK_UI.offsetX;
  const pileHeight = cardHeight + Math.max(0, stack.length - 1) * Math.abs(STACK_UI.offsetY);

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

  if (stack.length === 0 && !flashCard) return null;

  return (
    <div
      className={cn("pointer-events-auto", STACK_UI.positionClass)}
      style={{ top }}
    >
      <div
        className="relative"
        style={{ height: `${pileHeight}px`, width: `${pileWidth}px` }}
        onMouseLeave={() => setHoveredId(null)}
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
              onMouseEnter={() => setHoveredId(obj.id)}
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

import React, { useRef, useState, useLayoutEffect, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import type { Card as XMageCard } from "@/types/xmage";
import { Move, MousePointer2 } from "lucide-react";

const CARD_W = 70;
const CARD_H = 98;
const GAP = 8;

interface FreeBattlefieldProps {
  cards: XMageCard[];
  label: string;
  className?: string;
  onClickCard?: (card: XMageCard) => void;
  onClickAnyCard?: (card: XMageCard) => void;
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent) => void;
  onFlipCard?: () => void;
  showBackFace?: boolean;
  pendingCardIds?: string[];
  attackingCardIds?: string[];
  tappableLandIds?: string[];
  onTapLand?: (card: XMageCard) => void;
  untappableLandIds?: string[];
  onUntapLand?: (card: XMageCard) => void;
  /** Reserve this many px at the bottom for the hand overlay (clamps drag + auto-layout) */
  bottomReserved?: number;
  /** When true, renders a green dashed drop-target overlay */
  isDropActive?: boolean;
}

interface Marquee {
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
  additive: boolean; // shift held during marquee → add to existing selection
}

export function FreeBattlefield({
  cards,
  label,
  className,
  onClickCard,
  onClickAnyCard,
  onHoverCard,
  onFlipCard,
  showBackFace,
  pendingCardIds,
  attackingCardIds,
  tappableLandIds,
  onTapLand,
  untappableLandIds,
  onUntapLand,
  bottomReserved = 0,
  isDropActive = false,
}: FreeBattlefieldProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [positions, setPositions] = useState<Record<string, { x: number; y: number }>>({});
  const [selectedCardIds, setSelectedCardIds] = useState<Set<string>>(new Set());
  const [draggingCardIds, setDraggingCardIds] = useState<Set<string>>(new Set());
  const [selectMode, setSelectMode] = useState(false);
  const [marquee, setMarquee] = useState<Marquee | null>(null);

  // Refs so event handlers always read the latest state without stale closures
  const positionsRef = useRef(positions);
  positionsRef.current = positions;
  const selectedCardIdsRef = useRef(selectedCardIds);
  selectedCardIdsRef.current = selectedCardIds;
  const selectModeRef = useRef(selectMode);
  selectModeRef.current = selectMode;
  // Tracks the live marquee during drag without relying on state reads inside closures
  const marqueeRef = useRef<Marquee | null>(null);

  // Mutable drag state — updated without triggering re-renders
  const dragRef = useRef<{
    cardIds: string[];
    startMouseX: number;
    startMouseY: number;
    startPositions: Record<string, { x: number; y: number }>;
    moved: boolean;
  } | null>(null);

  // Auto-position new cards in a left-to-right grid; remove departed cards from
  // positions and selection
  useLayoutEffect(() => {
    if (!containerRef.current) return;
    const containerW = containerRef.current.clientWidth;
    const containerH = containerRef.current.clientHeight;
    const cols = Math.max(1, Math.floor((containerW + GAP) / (CARD_W + GAP)));
    // Don't auto-place new cards into the reserved zone at the bottom
    const yMax = containerH > 0 ? Math.max(0, containerH - CARD_H - bottomReserved) : Infinity;

    setPositions((prev) => {
      const next = { ...prev };
      const cardIds = new Set(cards.map((c) => c.id));
      for (const id of Object.keys(next)) {
        if (!cardIds.has(id)) delete next[id];
      }
      const alreadyPositioned = Object.keys(next).length;
      let newIdx = 0;
      for (const card of cards) {
        if (!next[card.id]) {
          const slot = alreadyPositioned + newIdx;
          next[card.id] = {
            x: (slot % cols) * (CARD_W + GAP) + GAP,
            y: Math.min(Math.floor(slot / cols) * (CARD_H + GAP) + GAP, yMax),
          };
          newIdx++;
        }
      }
      return next;
    });

    setSelectedCardIds((prev) => {
      const cardIds = new Set(cards.map((c) => c.id));
      const next = new Set([...prev].filter((id) => cardIds.has(id)));
      return next.size === prev.size ? prev : next;
    });
  }, [cards]);

  // Card mousedown: shift+click toggles selection; otherwise start drag
  const handleMouseDown = useCallback((e: React.MouseEvent, cardId: string) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation(); // prevent container handler from firing

    // Shift+click: toggle this card in/out of the selection, no drag
    if (e.shiftKey) {
      setSelectedCardIds((prev) => {
        const next = new Set(prev);
        if (next.has(cardId)) next.delete(cardId);
        else next.add(cardId);
        return next;
      });
      return;
    }

    const pos = positionsRef.current[cardId];
    if (!pos) return;

    // If dragging a card that's part of the current selection, move all of them;
    // otherwise clear the selection and drag just this card
    const inSelection = selectedCardIdsRef.current.has(cardId);
    const cardsToDrag = inSelection ? [...selectedCardIdsRef.current] : [cardId];

    if (!inSelection) setSelectedCardIds(new Set());

    const startPositions: Record<string, { x: number; y: number }> = {};
    for (const id of cardsToDrag) {
      startPositions[id] = positionsRef.current[id] ?? { x: 0, y: 0 };
    }

    dragRef.current = {
      cardIds: cardsToDrag,
      startMouseX: e.clientX,
      startMouseY: e.clientY,
      startPositions,
      moved: false,
    };
    setDraggingCardIds(new Set(cardsToDrag));

    const handleMouseMove = (me: MouseEvent) => {
      if (!dragRef.current) return;
      const dx = me.clientX - dragRef.current.startMouseX;
      const dy = me.clientY - dragRef.current.startMouseY;
      if (!dragRef.current.moved && Math.sqrt(dx * dx + dy * dy) < 5) return;
      dragRef.current.moved = true;

      const el = containerRef.current;
      if (!el) return;

      setPositions((prev) => {
        const next = { ...prev };
        for (const id of dragRef.current!.cardIds) {
          const start = dragRef.current!.startPositions[id];
          if (!start) continue;
          next[id] = {
            x: Math.max(0, Math.min(el.clientWidth - CARD_W, start.x + dx)),
            y: Math.max(0, Math.min(el.clientHeight - CARD_H, start.y + dy)),
          };
        }
        return next;
      });
    };

    const handleMouseUp = () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      setDraggingCardIds(new Set());
      dragRef.current = null;
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, []);

  // Container background mousedown: marquee in select mode, clear selection otherwise
  const handleContainerMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;

    const el = containerRef.current;
    if (!el) return;

    if (selectModeRef.current) {
      e.preventDefault();
      const rect = el.getBoundingClientRect();
      const startX = e.clientX - rect.left;
      const startY = e.clientY - rect.top;
      const additive = e.shiftKey; // captured at mousedown time

      const initial: Marquee = { startX, startY, currentX: startX, currentY: startY, additive };
      marqueeRef.current = initial;
      setMarquee(initial);

      const handleMouseMove = (me: MouseEvent) => {
        const currentX = Math.max(0, Math.min(el.clientWidth, me.clientX - rect.left));
        const currentY = Math.max(0, Math.min(el.clientHeight, me.clientY - rect.top));
        const updated = { ...marqueeRef.current!, currentX, currentY };
        marqueeRef.current = updated;
        setMarquee(updated);
      };

      const handleMouseUp = () => {
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);

        const m = marqueeRef.current;
        marqueeRef.current = null;
        setMarquee(null);

        if (!m) return;
        const selX = Math.min(m.startX, m.currentX);
        const selY = Math.min(m.startY, m.currentY);
        const selW = Math.abs(m.currentX - m.startX);
        const selH = Math.abs(m.currentY - m.startY);

        if (selW > 4 || selH > 4) {
          const hits = new Set<string>();
          for (const [id, pos] of Object.entries(positionsRef.current)) {
            if (
              pos.x < selX + selW &&
              pos.x + CARD_W > selX &&
              pos.y < selY + selH &&
              pos.y + CARD_H > selY
            ) {
              hits.add(id);
            }
          }
          setSelectedCardIds(
            additive ? new Set([...selectedCardIdsRef.current, ...hits]) : hits,
          );
        } else if (!additive) {
          // Plain click on empty space: clear selection
          setSelectedCardIds(new Set());
        }
      };

      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    } else {
      // Normal mode: background click clears selection
      setSelectedCardIds(new Set());
    }
  }, []);

  // Minimum container height to show all positioned cards
  const minH = Math.max(
    120,
    ...cards.map((c) => {
      const p = positions[c.id];
      return p ? p.y + CARD_H + GAP : 120;
    }),
  );

  // Render the marquee rect
  const marqueeRect = marquee
    ? {
        left: Math.min(marquee.startX, marquee.currentX),
        top: Math.min(marquee.startY, marquee.currentY),
        width: Math.abs(marquee.currentX - marquee.startX),
        height: Math.abs(marquee.currentY - marquee.startY),
      }
    : null;

  return (
    <div className={cn("flex flex-col gap-1 min-h-0 flex-1", className)}>
      {/* Label row with mode toggle */}
      <div className="flex items-center justify-between px-1">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          {label}
          {selectedCardIds.size > 0 && (
            <span className="ml-2 text-purple-400 normal-case font-normal tracking-normal">
              ({selectedCardIds.size} selected)
            </span>
          )}
        </span>
        <div className="flex gap-0.5">
          <button
            title="Move mode"
            onClick={() => setSelectMode(false)}
            className={cn(
              "p-0.5 rounded transition-colors",
              !selectMode
                ? "text-foreground bg-muted"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            <Move size={12} />
          </button>
          <button
            title="Select mode — drag to rubber-band select cards"
            onClick={() => setSelectMode(true)}
            className={cn(
              "p-0.5 rounded transition-colors",
              selectMode
                ? "text-foreground bg-muted"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            <MousePointer2 size={12} />
          </button>
        </div>
      </div>

      {/* Battlefield canvas */}
      <div
        ref={containerRef}
        className={cn(
          "relative border rounded-lg bg-muted/20 overflow-hidden flex-1",
          selectMode && "cursor-crosshair",
        )}
        style={{ minHeight: `${minH}px` }}
        onMouseDown={handleContainerMouseDown}
      >
        {cards.length === 0 && (
          <span className="absolute inset-0 flex items-center justify-center text-xs text-muted-foreground italic">
            No permanents
          </span>
        )}

        {cards.map((card) => {
          const pos = positions[card.id] ?? { x: 0, y: 0 };
          const isPending = pendingCardIds?.includes(card.id);
          const isAttacking = attackingCardIds?.includes(card.id);
          const isTappable = tappableLandIds?.includes(card.id);
          const isUntappable = untappableLandIds?.includes(card.id);
          const isChoosableClick =
            (card.isChoosable && !!onClickCard) ||
            (isAttacking && !!onClickAnyCard);
          const isDragging = draggingCardIds.has(card.id);
          const isSelected = selectedCardIds.has(card.id);

          return (
            <div
              key={card.id}
              data-card-id={card.id}
              className="absolute group"
              style={{
                left: pos.x,
                top: pos.y,
                width: CARD_W,
                zIndex: isDragging ? 100 : isSelected ? 10 : 1,
              }}
              onMouseDown={(e) => handleMouseDown(e, card.id)}
              onMouseEnter={(e) => onHoverCard?.(card, e)}
              onMouseLeave={() => onHoverCard?.(null)}
            >
              <Card
                card={card}
                isTapped={card.tapped}
                onFlip={onFlipCard}
                showBackFace={showBackFace}
                className={cn(
                  "w-[70px] h-[98px] shrink-0",
                  isDragging ? "cursor-grabbing" : "cursor-grab",
                  // Selection ring takes priority; game rings shown when not selected
                  isSelected && "ring-2 ring-purple-400",
                  !isSelected && card.isChoosable && onClickCard && "ring-2 ring-blue-400",
                  !isSelected && isPending && "ring-2 ring-orange-400",
                  !isSelected && isAttacking && "ring-2 ring-red-500",
                  !isSelected && isTappable && !isAttacking && "ring-2 ring-yellow-400",
                  !isSelected && isUntappable && !isAttacking && !isTappable && "ring-2 ring-cyan-400",
                )}
              />

              {/* Tap-for-mana overlay */}
              {isTappable && onTapLand && (
                <button
                  className="absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 bg-yellow-400/20 border-2 border-yellow-400 transition-opacity flex items-end justify-center pb-1"
                  onMouseDown={(e) => e.stopPropagation()}
                  onClick={() => onTapLand(card)}
                  title={`Tap ${card.name} for mana`}
                >
                  <span className="text-[9px] font-bold text-yellow-800 bg-yellow-200/90 px-1 rounded leading-none">
                    TAP
                  </span>
                </button>
              )}

              {/* Untap overlay */}
              {isUntappable && onUntapLand && (
                <button
                  className="absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 bg-cyan-400/20 border-2 border-cyan-400 transition-opacity flex items-end justify-center pb-1"
                  onMouseDown={(e) => e.stopPropagation()}
                  onClick={() => onUntapLand(card)}
                  title={`Untap ${card.name} (undo mana)`}
                >
                  <span className="text-[9px] font-bold text-cyan-900 bg-cyan-200/90 px-1 rounded leading-none">
                    UNTAP
                  </span>
                </button>
              )}

              {/* Choosable / attacker overlay */}
              {!isTappable && isChoosableClick && (
                <button
                  className={cn(
                    "absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 border-2 transition-opacity",
                    isPending
                      ? "bg-orange-500/20 border-orange-400"
                      : isAttacking
                        ? "bg-red-500/20 border-red-500"
                        : "bg-blue-500/20 border-blue-400",
                  )}
                  onMouseDown={(e) => e.stopPropagation()}
                  onClick={() => {
                    if (card.isChoosable && onClickCard) onClickCard(card);
                    else if (isAttacking && onClickAnyCard) onClickAnyCard(card);
                  }}
                  title={
                    isPending
                      ? `Deselect ${card.name}`
                      : isAttacking
                        ? `Block ${card.name}`
                        : `Select ${card.name}`
                  }
                />
              )}
            </div>
          );
        })}

        {/* Marquee selection rectangle */}
        {marqueeRect && (
          <div
            className="absolute pointer-events-none border-2 border-dashed border-purple-400 bg-purple-400/10 rounded"
            style={{
              left: marqueeRect.left,
              top: marqueeRect.top,
              width: marqueeRect.width,
              height: marqueeRect.height,
            }}
          />
        )}

        {/* Drop-to-play indicator */}
        {isDropActive && (
          <div className="absolute inset-0 border-2 border-dashed border-green-400 bg-green-400/5 rounded-lg pointer-events-none z-30 flex items-end justify-center pb-2">
            <span className="text-green-400 text-[10px] font-bold bg-green-950/80 px-2 py-0.5 rounded">
              Drop to play
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

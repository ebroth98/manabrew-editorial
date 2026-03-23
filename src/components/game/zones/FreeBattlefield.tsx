import React, { useEffect, useMemo } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { CardOverlayButton } from "@/components/game/CardOverlayButton";
import type { Card as XMageCard } from "@/types/openmagic";
import { Move, MousePointer2 } from "lucide-react";
import { CARD_W, CARD_H, CARD_GAP as GAP } from "../game.constants";
import { CARD_RING, BATTLEFIELD_CARD } from "../game.styles";
import { useGameThemeColors, withAlpha } from "../game.theme";
import { useBattlefieldLayout } from "@/hooks/useBattlefieldLayout";

const ATTACH_OFFSET_Y = 16;

export interface PlacementGhost {
  stackObjectId: string;
  cardName: string;
  controllerId: string;
}

interface FreeBattlefieldProps {
  cards: XMageCard[];
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
  bottomReserved?: number;
  leftReserved?: number;
  rightReserved?: number;
  isDropActive?: boolean;
  /** When set, renders a dotted ghost outline where a permanent will land. */
  placementGhost?: PlacementGhost | null;
  /** Whether the current targeting prompt is hostile (affects choosable highlight color). */
  hostileTargeting?: boolean;
}

export function FreeBattlefield({
  cards,
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
  leftReserved = 0,
  rightReserved = 0,
  isDropActive = false,
  placementGhost,
  hostileTargeting = false,
}: FreeBattlefieldProps) {
  const themeColors = useGameThemeColors();
  const cardMap = useMemo(() => {
    const m = new Map<string, XMageCard>();
    for (const c of cards) m.set(c.id, c);
    return m;
  }, [cards]);

  // IDs of cards that are attached to another card on the battlefield
  const attachedIds = useMemo(() => {
    const s = new Set<string>();
    for (const c of cards) {
      if (c.attachedTo && cardMap.has(c.attachedTo)) s.add(c.id);
    }
    return s;
  }, [cards, cardMap]);

  // Only top-level cards get grid positions
  const topLevelCards = useMemo(
    () => cards.filter((c) => !attachedIds.has(c.id)),
    [cards, attachedIds],
  );

  const topLevelIds = useMemo(() => topLevelCards.map((c) => c.id), [topLevelCards]);
  const landCardIds = useMemo(
    () => topLevelCards.filter((c) => c.types.includes("Land")).map((c) => c.id),
    [topLevelCards],
  );

  const {
    containerRef,
    positions,
    selectedCardIds,
    draggingCardIds,
    justDraggedCardIds,
    selectMode,
    setSelectMode,
    marqueeRect,
    handleCardMouseDown,
    handleContainerMouseDown,
  } = useBattlefieldLayout({ cardIds: topLevelIds, bottomReserved, leftReserved, rightReserved, landCardIds });

  const isDraggingAnyCard = draggingCardIds.size > 0;

  useEffect(() => {
    if (isDraggingAnyCard) {
      onHoverCard?.(null);
    }
  }, [isDraggingAnyCard, onHoverCard]);

  // Minimum container height to show all positioned cards (account for attachment offset)
  const minH = Math.max(
    120,
    ...topLevelCards.map((c) => {
      const p = positions[c.id];
      const attCount = (c.attachmentIds ?? []).filter((id) => cardMap.has(id)).length;
      const offset = attCount * ATTACH_OFFSET_Y;
      return p ? p.y + CARD_H + GAP + offset : 120;
    }),
  );

  const renderSingleCard = (
    card: XMageCard,
    opts: { onMouseDown?: (e: React.MouseEvent) => void; extraStyle?: React.CSSProperties } = {},
  ) => {
    const isPending = pendingCardIds?.includes(card.id);
    const isAttacking = attackingCardIds?.includes(card.id);
    const isTappable = tappableLandIds?.includes(card.id);
    const isUntappable = untappableLandIds?.includes(card.id);
    const isChoosableClick =
      (card.isChoosable && !!onClickCard) || (isAttacking && !!onClickAnyCard);
    const isDragging = draggingCardIds.has(card.id);
    const isSelected = selectedCardIds.has(card.id);
    const ringColor = isSelected
      ? themeColors.activeAction.active
      : isAttacking
        ? themeColors.promptAction.attackAction
        : isPending
          ? themeColors.promptAction.passAction
          : isTappable
            ? themeColors.activeAction.active
            : isUntappable
              ? themeColors.promptAction.cancel
              : isChoosableClick
                ? (hostileTargeting ? themeColors.arrow.hostileTarget : themeColors.promptAction.defenseAction)
                : null;

    return (
      <div
        key={card.id}
        data-card-id={card.id}
        className="absolute group"
        style={{
          width: CARD_W,
          ...opts.extraStyle,
        }}
        onMouseDown={(e) => {
          onHoverCard?.(null);
          opts.onMouseDown?.(e);
        }}
        onMouseEnter={(e) => {
          if (isDraggingAnyCard) {
            onHoverCard?.(null);
            return;
          }
          onHoverCard?.(card, e);
        }}
        onMouseLeave={() => onHoverCard?.(null)}
      >
        <Card
          card={card}
          isTapped={card.tapped}
          onFlip={onFlipCard}
          showBackFace={showBackFace}
          className={cn(
            BATTLEFIELD_CARD,
            isDragging ? "cursor-grabbing" : "cursor-grab",
            isSelected && CARD_RING.selected,
            !isSelected && card.isChoosable && onClickCard && CARD_RING.choosable,
            !isSelected && card.isChoosable && onClickCard && "choosable-pulse",
            !isSelected && isPending && CARD_RING.pending,
            !isSelected && isAttacking && CARD_RING.attacking,
            !isSelected && isTappable && !isAttacking && CARD_RING.tappable,
            !isSelected && isUntappable && !isAttacking && !isTappable && CARD_RING.untappable,
          )}
          style={ringColor ? ({
            "--tw-ring-color": ringColor,
            ...(card.isChoosable && onClickCard ? {
              "--choosable-ring-color": ringColor,
              "--choosable-glow-color": ringColor.replace(/[\d.]+\)$/, "0.3)"),
            } : {}),
          } as React.CSSProperties) : undefined}
        />

        {isTappable && onTapLand && (
          <CardOverlayButton
            variant="tap"
            label="TAP"
            onClick={() => {
              if (justDraggedCardIds.has(card.id)) return;
              onTapLand(card);
            }}
            title={`Tap ${card.name} for mana`}
          />
        )}

        {isUntappable && onUntapLand && (
          <CardOverlayButton
            variant="untap"
            label="UNTAP"
            onClick={() => {
              if (justDraggedCardIds.has(card.id)) return;
              onUntapLand(card);
            }}
            title={`Untap ${card.name} (undo mana)`}
          />
        )}

        {!isTappable && isChoosableClick && (
          <CardOverlayButton
            variant={isPending ? "pending" : isAttacking ? "attacking" : hostileTargeting ? "choosable-hostile" : "choosable"}
            onClick={() => {
              if (justDraggedCardIds.has(card.id)) return;
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
            stopMouseDown
          />
        )}
      </div>
    );
  };

  return (
    <div className={cn("flex flex-col gap-1 min-h-0 flex-1", className)}>
      <div
        ref={containerRef}
        className={cn(
          "relative border rounded-lg bg-muted/20 overflow-hidden flex-1",
          selectMode && "cursor-crosshair",
        )}
        style={{ minHeight: `${minH}px` }}
        onMouseDown={handleContainerMouseDown}
      >
        {/* Top-right tool controls */}
        <div className="absolute top-1 right-1 z-40 flex items-center gap-1">
          {selectedCardIds.size > 0 && (
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-card/90 border">
              {selectedCardIds.size} selected
            </span>
          )}
          <div className="flex gap-0.5 rounded bg-card/90 border p-0.5 shadow-sm">
            <button
              title="Move mode"
              onMouseDown={(e) => e.stopPropagation()}
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
              onMouseDown={(e) => e.stopPropagation()}
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

        {cards.length === 0 && (
          <span className="absolute inset-0 flex items-center justify-center text-xs text-muted-foreground italic">
            No permanents
          </span>
        )}

        {topLevelCards.map((card) => {
          const pos = positions[card.id] ?? { x: 0, y: 0 };
          const isDragging = draggingCardIds.has(card.id);
          const isSelected = selectedCardIds.has(card.id);
          const baseZ = isDragging ? 100 : isSelected ? 10 : 1;

          const attachments = (card.attachmentIds ?? [])
            .map((id) => cardMap.get(id))
            .filter((c): c is XMageCard => c !== undefined);

          const totalOffset = attachments.length * ATTACH_OFFSET_Y;

          return (
            <React.Fragment key={card.id}>
              {/* Attachments peeking above the host */}
              {attachments.map((att, i) =>
                renderSingleCard(att, {
                  extraStyle: {
                    left: pos.x,
                    top: pos.y + totalOffset - (attachments.length - i) * ATTACH_OFFSET_Y,
                    zIndex: baseZ + i,
                  },
                }),
              )}
              {/* Host card */}
              {renderSingleCard(card, {
                onMouseDown: (e) => handleCardMouseDown(e, card.id),
                extraStyle: {
                  left: pos.x,
                  top: pos.y + totalOffset,
                  zIndex: baseZ + attachments.length,
                },
              })}
            </React.Fragment>
          );
        })}

        {placementGhost && containerRef.current && (() => {
          const cw = containerRef.current!.clientWidth;
          const usableW = cw - leftReserved - rightReserved;
          const cols = Math.max(1, Math.floor((usableW + GAP) / (CARD_W + GAP)));
          const nonLandCount = topLevelCards.filter((c) => !c.types.includes("Land")).length;
          const slot = nonLandCount;
          const xMin = Math.max(0, leftReserved);
          const ghostX = Math.min(cw - CARD_W - rightReserved, xMin + (slot % cols) * (CARD_W + GAP) + GAP);
          const ghostY = Math.floor(slot / cols) * (CARD_H + GAP) + GAP;
          return (
            <div
              data-placement-ghost
              className="absolute pointer-events-none border-2 border-dashed rounded-lg"
              style={{
                left: ghostX,
                top: ghostY,
                width: CARD_W,
                height: CARD_H,
                borderColor: withAlpha(themeColors.activeAction.active, 0.55),
                backgroundColor: withAlpha(themeColors.activeAction.active, 0.08),
              }}
            >
              <span
                className="absolute inset-0 flex items-center justify-center text-[9px] font-medium text-center px-1 leading-tight"
                style={{ color: withAlpha(themeColors.activeAction.active, 0.7) }}
              >
                {placementGhost.cardName}
              </span>
            </div>
          );
        })()}

        {marqueeRect && (
          <div
            className="absolute pointer-events-none border-2 border-dashed rounded"
            style={{
              left: marqueeRect.left,
              top: marqueeRect.top,
              width: marqueeRect.width,
              height: marqueeRect.height,
              borderColor: themeColors.activeAction.active,
              backgroundColor: withAlpha(themeColors.activeAction.active, 0.1),
            }}
          />
        )}

        {isDropActive && (
          <div
            className="absolute inset-0 border-2 border-dashed rounded-lg pointer-events-none z-30 flex items-end justify-center pb-2"
            style={{
              borderColor: themeColors.activeAction.active,
              backgroundColor: withAlpha(themeColors.activeAction.active, 0.06),
            }}
          >
            <span
              className="text-[10px] font-bold px-2 py-0.5 rounded"
              style={{
                color: themeColors.activeAction.active,
                backgroundColor: withAlpha(themeColors.activeAction.priority, 0.28),
              }}
            >
              Drop to play
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

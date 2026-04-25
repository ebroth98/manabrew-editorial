import React, { useEffect, useMemo, useRef } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { CardOverlayButton } from "@/components/game/CardOverlayButton";
import type { Card as XMageCard, ActivatableAbilityInfo } from "@/types/openmagic";
import { CARD_W, CARD_H, CARD_GAP as GAP } from "../game.constants";
import { CARD_RING, BATTLEFIELD_CARD } from "../game.styles";
import { withAlpha } from "@/themes/gameTheme";
import { useTheme } from "@/hooks/useTheme";
import { useBattlefieldLayout } from "@/hooks/useBattlefieldLayout";
import { extractManaLetters, getExpandedManaAbilities } from "@/components/game/manaUtils";
import { ManaAbilityTapButton } from "./ManaAbilityTapButton";

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
  actionableCardIds?: string[];
  onClickAnyCard?: (card: XMageCard) => void;
  onHoverCard?: (
    card: XMageCard | null,
    e?: React.MouseEvent,
    options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect },
  ) => void;
  onFlipCard?: () => void;
  showBackFace?: boolean;
  pendingCardIds?: string[];
  attackingCardIds?: string[];
  tappableLandIds?: string[];
  onTapLand?: (card: XMageCard) => void;
  /** Tap multiple selected lands at once (queued). */
  onTapLands?: (cardIds: string[]) => void;
  /** Mana ability options for tappable lands (per-color tap buttons on dual lands). */
  manaAbilityOptions?: ActivatableAbilityInfo[];
  /** Tap a land with a specific mana ability (dual land color choice). */
  onTapLandAbility?: (cardId: string, abilityIndex: number, color?: string) => void;
  untappableLandIds?: string[];
  onUntapLand?: (card: XMageCard) => void;
  /** Untap multiple selected lands at once (queued). */
  onUntapLands?: (cardIds: string[]) => void;
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
  actionableCardIds,
  onClickAnyCard,
  onHoverCard,
  onFlipCard,
  showBackFace,
  pendingCardIds,
  attackingCardIds,
  tappableLandIds,
  onTapLand,
  onTapLands,
  manaAbilityOptions,
  onTapLandAbility,
  untappableLandIds,
  onUntapLand,
  onUntapLands,
  bottomReserved = 0,
  leftReserved = 0,
  rightReserved = 0,
  isDropActive = false,
  placementGhost,
  hostileTargeting = false,
}: FreeBattlefieldProps) {
  const themeColors = useTheme().gameTheme;
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
    marqueeRect,
    handleCardMouseDown,
    handleContainerMouseDown,
  } = useBattlefieldLayout({
    cardIds: topLevelIds,
    bottomReserved,
    leftReserved,
    rightReserved,
    landCardIds,
  });

  const expandedManaByCard = useMemo(() => {
    if (!tappableLandIds?.length || !manaAbilityOptions?.length)
      return new Map<string, ActivatableAbilityInfo[]>();
    const map = new Map<string, ActivatableAbilityInfo[]>();
    for (const id of tappableLandIds) {
      map.set(id, getExpandedManaAbilities(id, manaAbilityOptions));
    }
    return map;
  }, [tappableLandIds, manaAbilityOptions]);

  const isDraggingAnyCard = draggingCardIds.size > 0;
  const actionableCardIdSet = useMemo(() => new Set(actionableCardIds ?? []), [actionableCardIds]);

  const onHoverCardRef = useRef(onHoverCard);
  onHoverCardRef.current = onHoverCard;

  useEffect(() => {
    if (isDraggingAnyCard) {
      onHoverCardRef.current?.(null);
    }
  }, [isDraggingAnyCard]);

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
    const isActionable = actionableCardIdSet.has(card.id);
    const isChoosableClick =
      ((card.isChoosable || isActionable) && !!onClickCard) || (isAttacking && !!onClickAnyCard);
    const isDragging = draggingCardIds.has(card.id);
    const isSelected = selectedCardIds.has(card.id);
    const ringColor = isSelected
      ? themeColors.cardRing
      : isAttacking
        ? themeColors.promptAction.attackAction
        : isPending
          ? themeColors.promptAction.passAction
          : isTappable
            ? themeColors.cardRing
            : isUntappable
              ? themeColors.promptAction.cancel
              : isChoosableClick
                ? hostileTargeting
                  ? themeColors.arrow.hostileTarget
                  : themeColors.cardRing
                : null;

    /** Render a TAP or UNTAP overlay with multi-selection support. */
    const renderLandOverlay = (
      c: XMageCard,
      variant: "tap" | "untap",
      label: string,
      validIds: string[] | undefined,
      onSingle: (c: XMageCard) => void,
      onBatch: ((ids: string[]) => void) | undefined,
      titleFn: (name: string) => string,
    ) => (
      <CardOverlayButton
        variant={variant}
        label={label}
        onClick={() => {
          if (justDraggedCardIds.has(c.id)) return;
          if (selectedCardIds.has(c.id) && selectedCardIds.size > 1 && onBatch) {
            const batchIds = [...selectedCardIds].filter((id) => validIds?.includes(id));
            if (batchIds.length > 1) {
              onBatch(batchIds);
              return;
            }
          }
          onSingle(c);
        }}
        title={
          selectedCardIds.has(c.id) && selectedCardIds.size > 1
            ? `${label} ${selectedCardIds.size} selected lands`
            : titleFn(c.name)
        }
      />
    );

    return (
      <div
        key={card.id}
        data-card-id={card.id}
        className="absolute group transition-transform duration-150 ease-out hover:scale-105"
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
          onHoverCard?.(card, e, { useAnchor: true });
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
            !isSelected && isChoosableClick && CARD_RING.choosable,
            !isSelected && isChoosableClick && "choosable-pulse",
            !isSelected && isPending && CARD_RING.pending,
            !isSelected && isAttacking && CARD_RING.attacking,
            !isSelected && isTappable && !isAttacking && CARD_RING.tappable,
            !isSelected && isUntappable && !isAttacking && !isTappable && CARD_RING.untappable,
          )}
          style={
            ringColor
              ? ({
                  "--tw-ring-color": ringColor,
                  ...(isChoosableClick
                    ? {
                        "--choosable-ring-color": ringColor,
                        "--choosable-glow-color": withAlpha(ringColor, 0.3),
                      }
                    : {}),
                } as React.CSSProperties)
              : undefined
          }
        />

        {isTappable &&
          onTapLand &&
          (() => {
            const expanded = expandedManaByCard.get(card.id) ?? [];
            if (expanded.length > 1 && onTapLandAbility) {
              const isGrid = expanded.length > 2;
              return (
                <div
                  className={cn(
                    "absolute inset-0 z-20 overflow-hidden rounded-lg opacity-0 group-hover:opacity-100 transition-opacity",
                    isGrid ? "grid grid-cols-2 items-stretch" : "flex items-stretch justify-center",
                  )}
                >
                  {expanded.map((ab, idx) => {
                    const isLast = idx === expanded.length - 1;
                    const isOdd = expanded.length % 2 !== 0;
                    const shouldSpan = isGrid && isLast && isOdd;
                    return (
                      <ManaAbilityTapButton
                        key={`${ab.abilityIndex}-${idx}`}
                        description={ab.description}
                        small={isGrid}
                        className={shouldSpan ? "col-span-2" : ""}
                        onClick={() => {
                          if (justDraggedCardIds.has(card.id)) return;
                          const letters = extractManaLetters(ab.description);
                          onTapLandAbility(card.id, ab.abilityIndex, letters[0]);
                        }}
                      />
                    );
                  })}
                </div>
              );
            }
            return renderLandOverlay(
              card,
              "tap",
              "TAP",
              tappableLandIds,
              onTapLand,
              onTapLands,
              (name) => `Tap ${name} for mana`,
            );
          })()}

        {isUntappable &&
          onUntapLand &&
          renderLandOverlay(
            card,
            "untap",
            "UNTAP",
            untappableLandIds,
            onUntapLand,
            onUntapLands,
            (name) => `Untap ${name} (undo mana)`,
          )}

        {!isTappable && isChoosableClick && (
          <CardOverlayButton
            variant={
              isPending
                ? "pending"
                : isAttacking
                  ? "attacking"
                  : hostileTargeting
                    ? "choosable-hostile"
                    : "choosable"
            }
            onClick={() => {
              if (justDraggedCardIds.has(card.id)) return;
              if ((card.isChoosable || isActionable) && onClickCard) onClickCard(card);
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
  };

  return (
    <div className={cn("flex flex-col gap-1 min-h-0 flex-1", className)}>
      <div
        ref={containerRef}
        className="relative border rounded-lg bg-muted/20 overflow-hidden flex-1"
        style={{ minHeight: `${minH}px` }}
        onMouseDown={handleContainerMouseDown}
      >
        {/* Selection count badge */}
        {selectedCardIds.size > 0 && (
          <div className="absolute top-1 right-1 z-40">
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-card/90 border">
              {selectedCardIds.size} selected
            </span>
          </div>
        )}

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

        {placementGhost &&
          containerRef.current &&
          (() => {
            const cw = containerRef.current!.clientWidth;
            const usableW = cw - leftReserved - rightReserved;
            const cols = Math.max(1, Math.floor((usableW + GAP) / (CARD_W + GAP)));
            const xMin = Math.max(0, leftReserved);
            const xMax = Math.max(xMin, cw - CARD_W - Math.max(0, rightReserved));

            const isOccupied = (x: number, y: number) => {
              return Object.values(positions as any).some((pos: any) => {
                return (
                  x < pos.x + CARD_W + GAP / 2 &&
                  x + CARD_W + GAP / 2 > pos.x &&
                  y < pos.y + CARD_H + GAP / 2 &&
                  y + CARD_H + GAP / 2 > pos.y
                );
              });
            };

            let slot = 0;
            let ghostX = 0;
            let ghostY = 0;
            while (true) {
              ghostX = Math.min(xMax, xMin + (slot % cols) * (CARD_W + GAP) + GAP);
              ghostY = Math.floor(slot / cols) * (CARD_H + GAP) + GAP;
              if (!isOccupied(ghostX, ghostY) || slot > 200) break;
              slot++;
            }
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

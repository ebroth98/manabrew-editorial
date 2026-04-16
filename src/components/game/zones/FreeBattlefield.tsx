import React, { memo, useEffect, useMemo, useRef } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { CardOverlayButton } from "@/components/game/CardOverlayButton";
import type { Card as XMageCard, ActivatableAbilityInfo } from "@/types/openmagic";
import { CARD_W, CARD_H, CARD_GAP as GAP } from "../game.constants";
import { CARD_RING, BATTLEFIELD_CARD } from "../game.styles";
import { useGameThemeColors, withAlpha } from "../game.theme";
import { useBattlefieldLayout } from "@/hooks/useBattlefieldLayout";

const ATTACH_OFFSET_Y = 16;

/** Extract all mana letters from an ability description like "Add {G}." or "Add {W} or {U}." */
export function extractManaLetters(desc: string): string[] {
  const matches = desc.matchAll(/\{([WUBRGC])\}/g);
  return Array.from(matches, (m) => m[1]);
}

export function manaSymbolUrl(symbol: string): string {
  return `https://svgs.scryfall.io/card-symbols/${encodeURIComponent(symbol)}.svg`;
}

export const ANY_COLOR_LETTERS = ["W", "U", "B", "R", "G"];

/**
 * Expand mana abilities by detecting color options in descriptions.
 * - If it has multiple symbols (e.g. "{W} or {U}"), returns 2 virtual buttons.
 * - If it has NO symbols but says "any color", returns 5 virtual buttons (WUBRG).
 * - Otherwise returns the ability as-is (handles already-split abilities).
 */
export function getExpandedManaAbilities(
  cardId: string,
  options: ActivatableAbilityInfo[],
): ActivatableAbilityInfo[] {
  const cardAbs = options.filter((a) => a.cardId === cardId);
  if (cardAbs.length === 0) return [];

  const expanded: ActivatableAbilityInfo[] = [];

  for (const ab of cardAbs) {
    const letters = extractManaLetters(ab.description);
    const desc = ab.description.toLowerCase();
    const isAnyColor =
      desc.includes("any color") ||
      desc.includes("any one color") ||
      desc.includes("mana of any color");

    if (letters.length > 1) {
      // e.g. "Add {W} or {U}"
      letters.forEach((letter) => {
        expanded.push({
          ...ab,
          description: `Add {${letter}}`,
        });
      });
    } else if (letters.length === 1) {
      // Already has exactly one color symbol (e.g. "Add {W}").
      // Trust the symbol and don't expand further even if "any color" is in the text.
      // This prevents 25 buttons on cards where the engine already split the abilities.
      expanded.push(ab);
    } else if (isAnyColor) {
      // No specific symbols found but wording implies any color, expand to WUBRG
      ANY_COLOR_LETTERS.forEach((letter) => {
        expanded.push({
          ...ab,
          description: `Add {${letter}}`,
        });
      });
    } else {
      expanded.push(ab);
    }
  }

  return expanded;
}

export const MANA_COLORS: Record<string, string> = {
  W: "rgba(248, 246, 216, 0.45)", // White
  U: "rgba(193, 215, 233, 0.45)", // Blue
  B: "rgba(186, 177, 171, 0.45)", // Black
  R: "rgba(235, 159, 130, 0.45)", // Red
  G: "rgba(196, 211, 202, 0.45)", // Green
  C: "rgba(204, 202, 199, 0.45)", // Colorless
};

/** A button with a mana symbol for tapping a dual land for a specific color, styled to fill card sections. */
export const ManaAbilityTapButton = memo(function ManaAbilityTapButton({
  description,
  onClick,
  small = false,
  className,
}: {
  description: string;
  onClick: () => void;
  small?: boolean;
  className?: string;
}) {
  const letters = extractManaLetters(description);
  const letter = letters[0] ?? null;
  const bgColor = letter ? MANA_COLORS[letter] : "rgba(0, 0, 0, 0.4)";

  return (
    <button
      className={cn(
        "group/mana flex h-full w-full items-center justify-center transition-all hover:brightness-125",
        className,
      )}
      style={{ backgroundColor: bgColor }}
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
      onMouseDown={(e) => e.preventDefault()}
      title={`Tap: ${description}`}
    >
      <div
        className={cn(
          "flex items-center justify-center rounded-full bg-black/40 shadow-lg transition-transform group-hover/mana:scale-110",
          small ? "h-6 w-6 p-0.5" : "h-8 w-8 p-1",
        )}
      >
        {letter ? (
          <img
            src={manaSymbolUrl(letter)}
            alt={`{${letter}}`}
            className="h-full w-full drop-shadow-md"
            loading="lazy"
          />
        ) : (
          <span className={cn("font-bold text-white", small ? "text-[7px]" : "text-[9px]")}>
            TAP
          </span>
        )}
      </div>
    </button>
  );
});

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
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent, options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect }) => void;
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
    marqueeRect,
    handleCardMouseDown,
    handleContainerMouseDown,
  } = useBattlefieldLayout({ cardIds: topLevelIds, bottomReserved, leftReserved, rightReserved, landCardIds });

  const expandedManaByCard = useMemo(() => {
    if (!tappableLandIds?.length || !manaAbilityOptions?.length) return new Map<string, ActivatableAbilityInfo[]>();
    const map = new Map<string, ActivatableAbilityInfo[]>();
    for (const id of tappableLandIds) {
      map.set(id, getExpandedManaAbilities(id, manaAbilityOptions));
    }
    return map;
  }, [tappableLandIds, manaAbilityOptions]);

  const isDraggingAnyCard = draggingCardIds.size > 0;

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
    const isChoosableClick =
      (card.isChoosable && !!onClickCard) || (isAttacking && !!onClickAnyCard);
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
                ? (hostileTargeting ? themeColors.arrow.hostileTarget : themeColors.cardRing)
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
            if (batchIds.length > 1) { onBatch(batchIds); return; }
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
              "--choosable-glow-color": withAlpha(ringColor, 0.3),
            } : {}),
          } as React.CSSProperties) : undefined}
        />

        {isTappable && onTapLand && (() => {
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
            card, "tap", "TAP", tappableLandIds, onTapLand, onTapLands,
            (name) => `Tap ${name} for mana`,
          );
        })()}

        {isUntappable && onUntapLand && renderLandOverlay(
          card, "untap", "UNTAP", untappableLandIds, onUntapLand, onUntapLands,
          (name) => `Untap ${name} (undo mana)`,
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

        {placementGhost && containerRef.current && (() => {
          const cw = containerRef.current!.clientWidth;
          const usableW = cw - leftReserved - rightReserved;
          const cols = Math.max(1, Math.floor((usableW + GAP) / (CARD_W + GAP)));
          const xMin = Math.max(0, leftReserved);
          const xMax = Math.max(xMin, cw - CARD_W - Math.max(0, rightReserved));

          const isOccupied = (x: number, y: number) => {
            return Object.values(positions).some((pos) => {
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

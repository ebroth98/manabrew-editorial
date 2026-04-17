import { useMemo, useState, type CSSProperties } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { CardOverlayButton } from "@/components/game/CardOverlayButton";
import type { BattlefieldZoneProps } from "../game.types";
import { CARD_RING, BATTLEFIELD_CARD, ZONE_LABEL } from "../game.styles";
import { useGameThemeColors, withAlpha } from "../game.theme";
import type { Card as XMageCard } from "@/types/openmagic";
import { extractManaLetters, getExpandedManaAbilities } from "@/components/game/manaUtils";
import { ManaAbilityTapButton } from "./ManaAbilityTapButton";

const ATTACH_OFFSET_Y = 16;

export function BattlefieldZone({
  cards,
  label,
  emptyLabel,
  className,
  zoneBg,
  minHeight = 100,
  topReserved = 0,
  onClickCard,
  onClickAnyCard,
  onHoverCard,
  onFlipCard,
  showBackFace,
  pendingCardIds,
  attackingCardIds,
  tappableLandIds,
  onTapLand,
  manaAbilityOptions,
  onTapLandAbility,
  untappableLandIds,
  onUntapLand,
  leftReserved = 0,
  rightReserved = 0,
  landsAtTop = false,
  placementGhost,
  hostileTargeting = false,
}: BattlefieldZoneProps) {
  const [hoveredCardId, setHoveredCardId] = useState<string | null>(null);
  const themeColors = useGameThemeColors();

  const cardMap = useMemo(() => {
    const m = new Map<string, XMageCard>();
    for (const c of cards) m.set(c.id, c);
    return m;
  }, [cards]);

  // Cards that are attached to another card — will render with their host
  const attachedIds = useMemo(() => {
    const s = new Set<string>();
    for (const c of cards) {
      if (c.attachedTo && cardMap.has(c.attachedTo)) s.add(c.id);
    }
    return s;
  }, [cards, cardMap]);

  // Top-level cards (not attached to anything on the battlefield)
  const topLevel = useMemo(
    () => cards.filter((c) => !attachedIds.has(c.id)),
    [cards, attachedIds],
  );
  const nonLands = useMemo(() => topLevel.filter((c) => !c.types.includes("Land")), [topLevel]);
  const lands = useMemo(() => topLevel.filter((c) => c.types.includes("Land")), [topLevel]);

  const expandedManaByCard = useMemo(() => {
    if (!tappableLandIds?.length || !manaAbilityOptions?.length) return new Map<string, import("@/types/openmagic").ActivatableAbilityInfo[]>();
    const map = new Map<string, import("@/types/openmagic").ActivatableAbilityInfo[]>();
    for (const id of tappableLandIds) {
      map.set(id, getExpandedManaAbilities(id, manaAbilityOptions));
    }
    return map;
  }, [tappableLandIds, manaAbilityOptions]);

  const renderSingleCard = (card: XMageCard, extraClass?: string) => {
    const isPending = pendingCardIds?.includes(card.id);
    const isAttacking = attackingCardIds?.includes(card.id);
    const isTappable = tappableLandIds?.includes(card.id);
    const isUntappable = untappableLandIds?.includes(card.id);
    const isChoosableClick =
      (card.isChoosable && !!onClickCard) || (isAttacking && !!onClickAnyCard);
    const ringColor = isAttacking
      ? themeColors.promptAction.attackAction
      : isPending
        ? themeColors.promptAction.passAction
        : isTappable
          ? themeColors.cardRing
          : isUntappable            ? themeColors.promptAction.cancel
            : isChoosableClick
              ? (hostileTargeting ? themeColors.arrow.hostileTarget : themeColors.cardRing)
              : null;
    return (
      <div
        key={card.id}
        data-card-id={card.id}
        className={cn("relative group shrink-0", extraClass)}
        onMouseEnter={(e) => {
          setHoveredCardId(card.id);
          onHoverCard?.(card, e, { useAnchor: true });
        }}
        onMouseLeave={() => {
          setHoveredCardId(null);
          onHoverCard?.(null);
        }}
      >
        <Card
          card={card}
          isTapped={card.tapped}
          isHovered={hoveredCardId === card.id}
          onFlip={onFlipCard}
          showBackFace={showBackFace}
          className={cn(
            BATTLEFIELD_CARD,
            "hover:z-10",
            card.isChoosable && onClickCard && CARD_RING.choosable,
            card.isChoosable && onClickCard && "choosable-pulse",
            isPending && CARD_RING.pending,
            isAttacking && CARD_RING.attacking,
            isTappable && !isAttacking && CARD_RING.tappable,
            isUntappable && !isAttacking && !isTappable && CARD_RING.untappable,
          )}
          style={ringColor ? ({
            "--tw-ring-color": ringColor,
            ...(card.isChoosable && onClickCard ? {
              "--choosable-ring-color": ringColor,
              "--choosable-glow-color": withAlpha(ringColor, 0.3),
            } : {}),
          } as CSSProperties) : undefined}
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
                        const letters = extractManaLetters(ab.description);
                        onTapLandAbility(card.id, ab.abilityIndex, letters[0]);
                      }}
                    />
                  );
                })}
              </div>
            );
          }
          return (
            <CardOverlayButton

              variant="tap"
              label="TAP"
              onClick={() => onTapLand(card)}
              title={`Tap ${card.name} for mana`}
            />
          );
        })()}
        {isUntappable && onUntapLand && (
          <CardOverlayButton
            variant="untap"
            label="UNTAP"
            onClick={() => onUntapLand(card)}
            title={`Untap ${card.name} (undo mana)`}
          />
        )}
        {!isTappable && isChoosableClick && (
          <CardOverlayButton
            variant={isPending ? "pending" : isAttacking ? "attacking" : hostileTargeting ? "choosable-hostile" : "choosable"}
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
  };

  const renderCard = (card: XMageCard) => {
    const attachments = (card.attachmentIds ?? [])
      .map((id) => cardMap.get(id))
      .filter((c): c is XMageCard => c !== undefined);

    if (attachments.length === 0) {
      return renderSingleCard(card, "p-px");
    }

    // Stacked group: attachments peek out above the host
    const totalOffset = attachments.length * ATTACH_OFFSET_Y;
    return (
      <div
        key={card.id}
        className="relative shrink-0"
        style={{ paddingTop: totalOffset }}
      >
        {attachments.map((att, i) => (
          <div
            key={att.id}
            className="absolute left-0"
            style={{ top: i * ATTACH_OFFSET_Y, zIndex: i + 1 }}
          >
            {renderSingleCard(att)}
          </div>
        ))}
        <div className="relative" style={{ zIndex: attachments.length + 1 }}>
          {renderSingleCard(card)}
        </div>
      </div>
    );
  };

  const ghostEl = placementGhost ? (
    <div
      data-placement-ghost
      className="shrink-0 border-2 border-dashed rounded-lg flex items-center justify-center"
      style={{
        width: 70,
        height: 98,
        borderColor: withAlpha(themeColors.activeAction.active, 0.55),
        backgroundColor: withAlpha(themeColors.activeAction.active, 0.08),
      }}
    >
      <span
        className="text-[9px] font-medium text-center px-1 leading-tight"
        style={{ color: withAlpha(themeColors.activeAction.active, 0.7) }}
      >
        {placementGhost.cardName}
      </span>
    </div>
  ) : null;

  return (
    <div className={cn("flex flex-col gap-1 min-h-0", className)}>
      {label && <span className={ZONE_LABEL}>{label}</span>}
      <div
        className={cn("flex flex-col p-2 border rounded-lg flex-1", zoneBg ?? "bg-muted/20")}
        style={{
          minHeight: `${minHeight}px`,
          paddingTop: `${8 + topReserved}px`,
          paddingLeft: `${8 + leftReserved}px`,
          paddingRight: `${8 + rightReserved}px`,
        }}
      >
        {cards.length === 0 && !placementGhost ? (
          <span className="text-xs text-muted-foreground italic self-center mx-auto mt-auto mb-auto">
            {emptyLabel}
          </span>
        ) : (
          <>
            {landsAtTop ? (
              <>
                {lands.length > 0 && (
                  <div className="flex flex-wrap gap-2 pb-1">
                    {lands.map(renderCard)}
                  </div>
                )}
                <div className="flex flex-wrap gap-2 mt-auto content-start">
                  {nonLands.map(renderCard)}
                  {ghostEl}
                </div>
              </>
            ) : (
              <>
                <div className="flex flex-wrap gap-2 content-start">
                  {nonLands.map(renderCard)}
                  {ghostEl}
                </div>
                {lands.length > 0 && (
                  <div className="flex flex-wrap gap-2 mt-auto pt-1">
                    {lands.map(renderCard)}
                  </div>
                )}
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}

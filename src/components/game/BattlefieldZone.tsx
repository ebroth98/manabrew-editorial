import { useMemo, useState } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { CardOverlayButton } from "@/components/game/CardOverlayButton";
import type { BattlefieldZoneProps } from "./game.types";
import { CARD_RING, BATTLEFIELD_CARD, ZONE_LABEL } from "./game.styles";

export function BattlefieldZone({
  cards,
  label,
  emptyLabel,
  className,
  zoneBg,
  minHeight = 100,
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
  leftReserved = 0,
  rightReserved = 0,
  landsAtTop = false,
}: BattlefieldZoneProps) {
  const [hoveredCardId, setHoveredCardId] = useState<string | null>(null);

  const nonLands = useMemo(() => cards.filter((c) => !c.types.includes("Land")), [cards]);
  const lands = useMemo(() => cards.filter((c) => c.types.includes("Land")), [cards]);

  const renderCard = (card: (typeof cards)[number]) => {
    const isPending = pendingCardIds?.includes(card.id);
    const isAttacking = attackingCardIds?.includes(card.id);
    const isTappable = tappableLandIds?.includes(card.id);
    const isUntappable = untappableLandIds?.includes(card.id);
    const isChoosableClick =
      (card.isChoosable && !!onClickCard) || (isAttacking && !!onClickAnyCard);
    return (
      <div
        key={card.id}
        data-card-id={card.id}
        className="relative group shrink-0 p-px"
        onMouseEnter={(e) => {
          setHoveredCardId(card.id);
          onHoverCard?.(card, e);
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
            isPending && CARD_RING.pending,
            isAttacking && CARD_RING.attacking,
            isTappable && !isAttacking && CARD_RING.tappable,
            isUntappable && !isAttacking && !isTappable && CARD_RING.untappable,
          )}
        />
        {isTappable && onTapLand && (
          <CardOverlayButton
            variant="tap"
            label="TAP"
            onClick={() => onTapLand(card)}
            title={`Tap ${card.name} for mana`}
          />
        )}
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
            variant={isPending ? "pending" : isAttacking ? "attacking" : "choosable"}
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

  return (
    <div className={cn("flex flex-col gap-1 min-h-0", className)}>
      {label && <span className={ZONE_LABEL}>{label}</span>}
      <div
        className={cn("flex flex-col p-2 border rounded-lg flex-1", zoneBg ?? "bg-muted/20")}
        style={{
          minHeight: `${minHeight}px`,
          paddingLeft: `${8 + leftReserved}px`,
          paddingRight: `${8 + rightReserved}px`,
        }}
      >
        {cards.length === 0 ? (
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
                </div>
              </>
            ) : (
              <>
                <div className="flex flex-wrap gap-2 content-start">
                  {nonLands.map(renderCard)}
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

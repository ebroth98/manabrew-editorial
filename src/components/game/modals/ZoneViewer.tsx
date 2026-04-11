import { cn } from "@/lib/utils";
import type { Card as XMageCard } from "@/types/openmagic";
import { Card } from "@/components/game/Card";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { HAND_CARD } from "../game.styles";
import { useCardPreview } from "@/hooks/useCardPreview";
import { HoverCardPreview } from "@/components/game/HoverCardPreview";
import { useGameThemeColors } from "../game.theme";
import { Modal } from "./Modal";
import type { CSSProperties } from "react";

interface ZoneViewerProps {
  title: string;
  cards: XMageCard[];
  onClose: () => void;
  onClickCard?: (cardId: string) => void;
}

export function ZoneViewer({
  title,
  cards,
  onClose,
  onClickCard,
}: ZoneViewerProps) {
  const preview = useCardPreview();

  const themeColors = useGameThemeColors();
  const ringColor = themeColors.cardRing;

  return (
    <Modal onClose={onClose}>
      <Modal.Header onClose={onClose}>
        <h2 className="font-semibold text-base">{title}</h2>
      </Modal.Header>

      <Modal.Body>
        {cards.length === 0 ? (
          <Modal.EmptyState />
        ) : (
          <div className="flex flex-wrap gap-2 content-start">
            {cards.map((card) => (
              <div
                key={card.id}
                className="shrink-0 relative flex flex-col gap-1"
                onMouseEnter={(e) => preview.handleMouseEnter(card, e)}
                onMouseLeave={preview.handleMouseLeave}

              >
                <Card
                  card={card}
                  className={cn(
                    HAND_CARD,
                    card.isPlayable && onClickCard && "ring-2",
                  )}
                  style={
                    card.isPlayable && onClickCard
                      ? ({ "--tw-ring-color": ringColor } as CSSProperties)
                      : undefined
                  }
                  onClick={
                    card.isPlayable && onClickCard
                      ? () => onClickCard(card.id)
                      : undefined
                  }
                />
                {(card.effectiveManaCost || card.manaCost) && (
                  <div className="min-h-5 flex items-center justify-center">
                    <div className="inline-flex items-center rounded bg-muted/70 px-1.5 py-0.5">
                      <ManaSymbols
                        cost={card.effectiveManaCost ?? card.manaCost}
                        size="sm"
                      />
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </Modal.Body>

      <HoverCardPreview preview={preview} />
    </Modal>
  );
}

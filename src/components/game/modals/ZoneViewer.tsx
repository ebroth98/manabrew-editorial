import type { Card as CardType } from "@/types/openmagic";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { Modal } from "./Modal";
import { cn } from "@/lib/utils";
import { useHoverPreview } from "@/hooks/useHoverPreview";
import { HAND_CARD } from "../game.styles";

interface ZoneViewerProps {
  title: string;
  cards: CardType[];
  onClose: () => void;
  onClickCard?: (cardId: string) => void;
}

export function ZoneViewer({ title, cards, onClose, onClickCard }: ZoneViewerProps) {
  const { hoveredCard, mousePos, onMouseEnter, onMouseLeave } = useHoverPreview();

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
                onMouseEnter={(e) => onMouseEnter(card, e)}
                onMouseLeave={onMouseLeave}
              >
                <Card
                  card={card}
                  className={cn(HAND_CARD, card.isPlayable && onClickCard && "ring-2 ring-green-400")}
                  onClick={card.isPlayable && onClickCard ? () => onClickCard(card.id) : undefined}
                />
                {(card.effectiveManaCost || card.manaCost) && (
                  <div className="min-h-5 flex items-center justify-center">
                    <div className="inline-flex items-center rounded bg-muted/70 px-1.5 py-0.5">
                      <ManaSymbols cost={card.effectiveManaCost ?? card.manaCost} size="sm" />
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </Modal.Body>

      {hoveredCard && (
        <CardPreview card={hoveredCard} mouseX={mousePos.x} mouseY={mousePos.y} />
      )}
    </Modal>
  );
}

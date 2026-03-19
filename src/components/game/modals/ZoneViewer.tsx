import type { Card as CardType } from "@/types/xmage";
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
                className="shrink-0 relative"
                onMouseEnter={(e) => onMouseEnter(card, e)}
                onMouseLeave={onMouseLeave}
              >
                <Card
                  card={card}
                  className={cn(HAND_CARD, card.isPlayable && onClickCard && "ring-2 ring-green-400")}
                />
                {card.isPlayable && onClickCard && (
                  <button
                    className="absolute inset-x-0 bottom-1 mx-auto text-[10px] font-bold bg-green-600 text-white rounded px-1 py-0.5 shadow hover:bg-green-500 transition-colors whitespace-nowrap"
                    style={{ width: "fit-content", minWidth: "3.5rem" }}
                    onClick={(e) => {
                      e.stopPropagation();
                      onClickCard(card.id);
                    }}
                  >
                    {card.flashbackCost ? (
                      <span className="inline-flex items-center gap-0.5">FB <ManaSymbols cost={card.flashbackCost} size="sm" /></span>
                    ) : card.isPlotted ? (
                      "PLOT CAST"
                    ) : card.isMadnessExiled && card.madnessCost ? (
                      <span className="inline-flex items-center gap-0.5">MADNESS <ManaSymbols cost={card.madnessCost} size="sm" /></span>
                    ) : card.isWarpExiled ? (
                      <span className="inline-flex items-center gap-0.5">CAST <ManaSymbols cost={card.manaCost} size="sm" /></span>
                    ) : (
                      "CAST"
                    )}
                  </button>
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

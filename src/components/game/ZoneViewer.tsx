import { useState } from "react";
import type { Card as CardType } from "@/types/xmage";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { Modal } from "@/components/game/Modal";

interface ZoneViewerProps {
  title: string;
  cards: CardType[];
  onClose: () => void;
  onClickCard?: (cardId: string) => void;
}

export function ZoneViewer({ title, cards, onClose, onClickCard }: ZoneViewerProps) {
  const [hoveredCard, setHoveredCard] = useState<CardType | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });

  function handleMouseEnter(card: CardType, e: React.MouseEvent) {
    setHoveredCard(card);
    setMousePos({ x: e.clientX, y: e.clientY });
  }

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
                onMouseEnter={(e) => handleMouseEnter(card, e)}
                onMouseLeave={() => setHoveredCard(null)}
              >
                <Card
                  card={card}
                  className={`w-[80px] h-[112px] ${card.isPlayable && onClickCard ? "ring-2 ring-green-400" : ""}`}
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
                    {card.flashbackCost ? <span className="inline-flex items-center gap-0.5">FB <ManaSymbols cost={card.flashbackCost} size="sm" /></span> : "CAST"}
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

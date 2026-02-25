import { Card } from "@/components/game/Card";
import type { Card as CardType } from "@/types/xmage";
import { CardPreview } from "@/components/game/CardPreview";
import { Modal } from "@/components/game/Modal";
import { useState } from "react";

interface ZoneTargetSelectorProps {
  title: string;
  cards: CardType[];
  validCardIds: string[];
  onSelect: (cardId: string) => void;
  onCancel: () => void;
}

export function ZoneTargetSelector({
  title,
  cards,
  validCardIds,
  onSelect,
  onCancel
}: ZoneTargetSelectorProps) {
  const [hoveredCard, setHoveredCard] = useState<CardType | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });

  function handleMouseEnter(card: CardType, e: React.MouseEvent) {
    setHoveredCard(card);
    setMousePos({ x: e.clientX, y: e.clientY });
  }

  const validCards = cards.filter(card => validCardIds.includes(card.id));

  return (
    <Modal onClose={onCancel} maxWidth="max-w-4xl" maxHeight="max-h-[85vh]">
      <Modal.Header onClose={onCancel}>
        <h2 className="font-semibold text-base">{title}</h2>
      </Modal.Header>

      <Modal.Instructions>Choose a target card</Modal.Instructions>

      <Modal.Body>
        {validCards.length === 0 ? (
          <Modal.EmptyState message="No valid targets in this zone" />
        ) : (
          <div className="flex flex-wrap gap-3 content-start justify-center">
            {validCards.map((card) => (
              <div
                key={card.id}
                className="shrink-0 cursor-pointer group"
                onMouseEnter={(e) => handleMouseEnter(card, e)}
                onMouseLeave={() => setHoveredCard(null)}
                onClick={() => onSelect(card.id)}
              >
                <Card
                  card={card}
                  className="w-[100px] h-[140px] transition-transform group-hover:scale-105 group-hover:-translate-y-2"
                />
                <div className="text-center mt-1">
                  <span className="text-xs text-muted-foreground">{card.name}</span>
                </div>
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

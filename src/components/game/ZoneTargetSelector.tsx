import { createPortal } from "react-dom";
import { Card } from "@/components/game/Card";
import type { Card as CardType } from "@/types/xmage";
import { CardPreview } from "@/components/game/CardPreview";
import { X } from "lucide-react";
import { useState, useEffect } from "react";

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

  // Close on Escape
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onCancel();
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [onCancel]);

  const validCards = cards.filter(card => validCardIds.includes(card.id));

  return createPortal(
    <div
      className="fixed inset-0 z-[9000] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onCancel}
    >
      <div
        className="bg-card border rounded-xl shadow-2xl flex flex-col max-w-4xl w-full max-h-[85vh] mx-4 animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <h2 className="font-semibold text-base">{title}</h2>
          <button
            className="rounded-md p-1 hover:bg-muted transition-colors"
            onClick={onCancel}
            title="Close (Esc)"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Instructions */}
        <div className="px-4 py-2 bg-blue-50 dark:bg-blue-950/20 border-b">
          <p className="text-sm font-semibold text-blue-700 dark:text-blue-400 text-center">
            Choose a target card
          </p>
        </div>

        {/* Card grid */}
        <div className="overflow-y-auto p-4 flex-1">
          {validCards.length === 0 ? (
            <p className="text-sm text-muted-foreground italic text-center py-8">
              No valid targets in this zone
            </p>
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
        </div>
      </div>

      {hoveredCard && (
        <CardPreview card={hoveredCard} mouseX={mousePos.x} mouseY={mousePos.y} />
      )}
    </div>,
    document.body
  );
}

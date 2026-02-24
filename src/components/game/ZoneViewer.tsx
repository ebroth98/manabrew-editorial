import { createPortal } from "react-dom";
import { useEffect, useState } from "react";
import type { Card as CardType } from "@/types/xmage";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { X } from "lucide-react";

interface ZoneViewerProps {
  title: string;
  cards: CardType[];
  onClose: () => void;
  onClickCard?: (cardId: string) => void;
}

export function ZoneViewer({ title, cards, onClose, onClickCard }: ZoneViewerProps) {
  const [hoveredCard, setHoveredCard] = useState<CardType | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });

  // Close on Escape
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [onClose]);

  function handleMouseEnter(card: CardType, e: React.MouseEvent) {
    setHoveredCard(card);
    setMousePos({ x: e.clientX, y: e.clientY });
  }

  return createPortal(
    <div
      className="fixed inset-0 z-[9000] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="bg-card border rounded-xl shadow-2xl flex flex-col max-w-2xl w-full max-h-[80vh] mx-4 animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <h2 className="font-semibold text-base">{title}</h2>
          <button
            className="rounded-md p-1 hover:bg-muted transition-colors"
            onClick={onClose}
            title="Close (Esc)"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Card grid */}
        <div className="overflow-y-auto p-4 flex-1">
          {cards.length === 0 ? (
            <p className="text-sm text-muted-foreground italic text-center py-8">No cards</p>
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
        </div>
      </div>

      {hoveredCard && (
        <CardPreview card={hoveredCard} mouseX={mousePos.x} mouseY={mousePos.y} />
      )}
    </div>,
    document.body
  );
}

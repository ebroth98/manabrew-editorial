import { createPortal } from "react-dom";
import { useEffect, useState } from "react";
import type { Card as CardType } from "@/types/xmage";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { X } from "lucide-react";

interface ZoneViewerProps {
  title: string;
  cards: CardType[];
  onClose: () => void;
}

export function ZoneViewer({ title, cards, onClose }: ZoneViewerProps) {
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
                  className="shrink-0"
                  onMouseEnter={(e) => handleMouseEnter(card, e)}
                  onMouseLeave={() => setHoveredCard(null)}
                >
                  <Card card={card} className="w-[80px] h-[112px]" />
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

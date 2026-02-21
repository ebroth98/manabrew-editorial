import { createPortal } from "react-dom";
import { useCardImage } from "@/hooks/useCardImage";
import { Loader2 } from "lucide-react";
import type { Card } from "@/types/xmage";
import { CounterDisplay } from "@/components/game/CounterBadge";

interface CardPreviewProps {
  card: Card;
  mouseX: number;
  mouseY: number;
}

const CARD_W = 240;
const CARD_H = 336; // 5:7 ratio

/**
 * Floating card preview rendered into document.body via portal.
 * Positions itself near the cursor, clamped to viewport edges.
 */
export function CardPreview({ card, mouseX, mouseY }: CardPreviewProps) {
  const { data: fetchedUrl, isLoading } = useCardImage(card.name, card.imageUrl, card.isToken, card.color);
  const imageUrl = card.imageUrl ?? fetchedUrl ?? null;

  // Determine horizontal placement: prefer right of cursor, flip left if near edge
  const spaceRight = window.innerWidth - mouseX;
  const left =
    spaceRight > CARD_W + 24
      ? mouseX + 16
      : mouseX - CARD_W - 16;

  // Clamp vertical so the card stays within viewport
  const top = Math.min(
    Math.max(mouseY - CARD_H / 2, 8),
    window.innerHeight - CARD_H - 8
  );

  return createPortal(
    <div
      className="fixed z-[9999] pointer-events-none select-none"
      style={{ left, top, width: CARD_W, height: CARD_H }}
    >
      <div className="w-full h-full rounded-xl shadow-2xl ring-1 ring-black/20 overflow-hidden bg-card">
        {isLoading && !imageUrl ? (
          <div className="w-full h-full flex flex-col items-center justify-center gap-2 p-4">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            <span className="text-xs text-muted-foreground text-center">{card.name}</span>
          </div>
        ) : imageUrl ? (
          <>
            <img
              src={imageUrl}
              alt={card.name}
              className="w-full h-full object-contain"
            />
            {card.counters && (
              <CounterDisplay
                counters={card.counters}
                size="md"
                className="absolute bottom-2 left-2 z-10"
              />
            )}
          </>
        ) : (
          // Text fallback for cards with no image
          <div className="w-full h-full p-4 flex flex-col gap-2 bg-card">
            <div className="flex justify-between items-start">
              <span className="font-bold text-sm leading-tight">{card.name}</span>
              <span className="text-xs font-mono text-muted-foreground">{card.manaCost}</span>
            </div>
            <div className="text-xs text-muted-foreground">{card.types?.join(" ")}</div>
            <div className="flex-1 text-xs text-foreground/80 whitespace-pre-wrap">{card.text}</div>
            {card.counters && (
              <CounterDisplay counters={card.counters} size="md" />
            )}
            {card.power && card.toughness && (
              <div className="text-right font-bold text-sm">
                {card.power}/{card.toughness}
              </div>
            )}
          </div>
        )}
      </div>
    </div>,
    document.body
  );
}

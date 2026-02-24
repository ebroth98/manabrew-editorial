import { createPortal } from "react-dom";
import { useCardImage } from "@/hooks/useCardImage";
import { Loader2 } from "lucide-react";
import type { Card } from "@/types/xmage";
import { CounterDisplay } from "@/components/game/CounterBadge";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { useQuery } from "@tanstack/react-query";
import { getCardByName } from "@/api/scryfall";

interface CardPreviewProps {
  card: Card;
  mouseX: number;
  mouseY: number;
  showBackFace?: boolean;
}

const CARD_W = 240;
const CARD_H = 336; // 5:7 ratio

/**
 * Floating card preview rendered into document.body via portal.
 * Positions itself near the cursor, clamped to viewport edges.
 * Supports showing front or back face for double-faced cards.
 */
export function CardPreview({ card, mouseX, mouseY, showBackFace = false }: CardPreviewProps) {
  const { data: fetchedUrl, isLoading } = useCardImage(card.name, card.imageUrl, card.isToken, card.color, card.setCode);
  const imageUrl = card.imageUrl ?? fetchedUrl ?? null;
  
  // Fetch double-faced card data if needed
  const { data: doubleFacedData } = useQuery({
    queryKey: ["double-faced-card", card.name, card.isDoubleFaced],
    queryFn: async () => {
      if (!card.isDoubleFaced) return null;
      const cardData = await getCardByName(card.name);
      
      // For double-faced cards, get both faces
      if (cardData.card_faces && cardData.card_faces.length >= 2) {
        const frontFace = cardData.card_faces[0];
        const backFace = cardData.card_faces[1];
        
        return {
          frontImageUrl: frontFace.image_uris?.normal ?? frontFace.image_uris?.large ?? null,
          backImageUrl: backFace.image_uris?.normal ?? backFace.image_uris?.large ?? null,
          frontName: frontFace.name,
          backName: backFace.name,
        };
      }
      return null;
    },
    enabled: !!card.isDoubleFaced,
    staleTime: Infinity,
    gcTime: 1000 * 60 * 60,
  });

  // Determine horizontal placement: prefer right of cursor, flip left if near edge
  const cardWidth = CARD_W;
  const cardHeight = CARD_H;
  
  const spaceRight = window.innerWidth - mouseX;
  const left =
    spaceRight > cardWidth + 24
      ? mouseX + 16
      : mouseX - cardWidth - 16;

  // Clamp vertical so the card stays within viewport
  const top = Math.min(
    Math.max(mouseY - cardHeight / 2, 8),
    window.innerHeight - cardHeight - 8
  );

  // Determine which image to show for double-faced cards
  const hasDoubleFace = !!card.isDoubleFaced && !!doubleFacedData?.backImageUrl;
  const currentImageUrl = hasDoubleFace && showBackFace 
    ? doubleFacedData.backImageUrl 
    : (imageUrl || fetchedUrl);
  const currentCardName = hasDoubleFace && showBackFace
    ? doubleFacedData.backName
    : (hasDoubleFace && !showBackFace ? doubleFacedData.frontName : card.name);

  return createPortal(
    <div
      className="fixed z-[9999] pointer-events-none select-none"
      style={{ left, top, width: cardWidth, height: cardHeight }}
    >
      <div className="relative w-full h-full">
        {/* Card display */}
        <div className="w-full h-full rounded-xl shadow-2xl ring-1 ring-black/20 overflow-hidden bg-card">
          {isLoading && !currentImageUrl ? (
            <div className="w-full h-full flex flex-col items-center justify-center gap-2 p-4">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              <span className="text-xs text-muted-foreground text-center">{currentCardName}</span>
            </div>
          ) : currentImageUrl ? (
            <>
              <img
                src={currentImageUrl}
                alt={currentCardName}
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
                <span className="font-bold text-sm leading-tight">{currentCardName}</span>
                {!hasDoubleFace && (
                  <ManaSymbols cost={card.manaCost} size="md" />
                )}
              </div>
              {!hasDoubleFace && (
                <div className="text-xs text-muted-foreground">{card.types?.join(" ")}</div>
              )}
              <div className="flex-1 text-xs text-foreground/80 whitespace-pre-wrap">
                {hasDoubleFace && showBackFace 
                  ? `Back face: ${doubleFacedData!.backName}` 
                  : (hasDoubleFace && !showBackFace 
                    ? `Front face: ${doubleFacedData!.frontName}` 
                    : card.text)}
              </div>
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
      </div>
    </div>,
    document.body
  );
}

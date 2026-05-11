import { Button } from "@/components/ui/button";
import { Modal } from "./Modal";
import { Card } from "@/components/game/Card";
import { useCard } from "@/stores/useScryfallStore";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { MODAL_CARD_THUMBNAIL, MODAL_FOOTER_BETWEEN } from "../game.styles";
import { useState, useCallback } from "react";
import { useModalKeyboard } from "@/hooks/useModalKeyboard";
import type { Card as CardType } from "@/types/manabrew";
import { cn } from "@/lib/utils";

interface ReorderLibraryModalProps {
  cards: CardType[];
  cardName?: string;
  onConfirm: (orderedCardIds: string[]) => void;
}

export function ReorderLibraryModal({ cards, cardName, onConfirm }: ReorderLibraryModalProps) {
  const cardData = useCard({ name: cardName ?? "" });
  const imageUrl = cardData?.uris.normal;
  // unsorted = cards not yet placed; sorted = placed in chosen order
  const [sorted, setSorted] = useState<CardType[]>([]);
  const unsorted = cards.filter((c) => !sorted.some((s) => s.id === c.id));
  const allSorted = sorted.length === cards.length;

  const handleClickUnsorted = (card: CardType) => {
    setSorted((prev) => [...prev, card]);
  };

  const handleClickSorted = (card: CardType) => {
    // Remove from sorted (put back in unsorted)
    setSorted((prev) => prev.filter((c) => c.id !== card.id));
  };

  const handleReset = () => {
    setSorted([]);
  };

  // Convention: last element = top of library
  const handleConfirm = useCallback(() => {
    if (allSorted) {
      onConfirm(sorted.map((c) => c.id));
    }
  }, [allSorted, sorted, onConfirm]);

  useModalKeyboard({ onEnter: allSorted ? handleConfirm : undefined }, [allSorted, handleConfirm]);

  return (
    <Modal maxWidth="max-w-lg" maxHeight="">
      <Modal.Header>
        <div className="flex items-center gap-3">
          {imageUrl && (
            <CardImageThumbnail
              imageUrl={imageUrl}
              cardName={cardName ?? "Source card"}
              className={MODAL_CARD_THUMBNAIL}
            />
          )}
          <div>
            <h2 className="font-semibold text-base">Reorder Top of Library</h2>
            {cardName && <p className="text-xs text-muted-foreground font-medium">{cardName}</p>}
          </div>
        </div>
      </Modal.Header>

      <Modal.Instructions>
        Click cards in the order you want them on your library. First clicked = bottom, last clicked
        = top.
      </Modal.Instructions>

      <div className="p-4 space-y-4">
        {/* Unsorted cards */}
        {unsorted.length > 0 && (
          <div>
            <p className="text-xs text-muted-foreground mb-2">Click to place:</p>
            <div className="flex gap-2 flex-wrap justify-center">
              {unsorted.map((card) => (
                <button
                  key={card.id}
                  onClick={() => handleClickUnsorted(card)}
                  className="cursor-pointer hover:ring-2 hover:ring-primary rounded transition-all"
                >
                  <Card card={card} />
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Sorted cards */}
        {sorted.length > 0 && (
          <div>
            <p className="text-xs text-muted-foreground mb-2">
              Library order (left = bottom, right = top):
            </p>
            <div className="flex gap-2 flex-wrap justify-center items-end">
              {sorted.map((card, i) => (
                <div key={card.id} className="flex flex-col items-center gap-1">
                  <span
                    className={cn(
                      "text-[10px] font-bold px-1.5 py-0.5 rounded",
                      i === sorted.length - 1
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted text-muted-foreground",
                    )}
                  >
                    {i === sorted.length - 1 ? "TOP" : i === 0 ? "BTM" : `${i + 1}`}
                  </span>
                  <button
                    onClick={() => handleClickSorted(card)}
                    className="cursor-pointer hover:ring-2 hover:ring-destructive rounded transition-all"
                    title="Click to remove"
                  >
                    <Card card={card} />
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className={MODAL_FOOTER_BETWEEN}>
        <Button size="sm" variant="ghost" onClick={handleReset} disabled={sorted.length === 0}>
          Reset
        </Button>
        <Button size="sm" disabled={!allSorted} onClick={handleConfirm}>
          Confirm Order
        </Button>
      </div>
    </Modal>
  );
}

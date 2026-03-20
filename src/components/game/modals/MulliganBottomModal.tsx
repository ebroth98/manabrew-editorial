import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Modal } from "./Modal";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { cn } from "@/lib/utils";
import { useState, useEffect, useCallback, useRef } from "react";
import { useModalKeyboard } from "@/hooks/useModalKeyboard";
import { useHoverPreview } from "@/hooks/useHoverPreview";
import { CARD_RING, MODAL_FOOTER_BETWEEN, MULLIGAN_CARD_SIZE } from "../game.styles";
import type { Card as CardType } from "@/types/openmagic";

interface MulliganBottomModalProps {
  handCards: CardType[];
  count: number;
  onConfirm: (cardIds: string[]) => void;
}

export function MulliganBottomModal({
  handCards,
  count,
  onConfirm,
}: MulliganBottomModalProps) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const canConfirm = selected.size === count;
  const dialogRef = useRef<HTMLDivElement>(null);
  const { hoveredCard, mousePos, onMouseEnter, onMouseLeave } = useHoverPreview();

  useEffect(() => {
    setSelected(new Set());
  }, [handCards]);

  useEffect(() => {
    dialogRef.current?.focus();
  }, [handCards]);

  const handleConfirm = useCallback(() => {
    if (canConfirm) onConfirm([...selected]);
  }, [selected, canConfirm, onConfirm]);

  function toggleCard(cardId: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(cardId)) {
        next.delete(cardId);
      } else {
        if (next.size >= count) return prev;
        next.add(cardId);
      }
      return next;
    });
  }

  useModalKeyboard(
    { onEnter: canConfirm ? handleConfirm : undefined },
    [canConfirm, handleConfirm],
  );

  return (
    <Modal maxWidth="max-w-[80vw]" maxHeight="max-h-[85vh]" className="w-fit">
      <div ref={dialogRef} tabIndex={-1} className="outline-none" role="dialog" aria-modal="true">
        <Modal.Header>
          <div className="flex items-center justify-between">
            <div>
              <h2 className="font-semibold text-base">Put Cards on Bottom</h2>
              <p className="text-xs text-muted-foreground">
                Choose {count} card{count !== 1 ? "s" : ""} to put on the bottom of your library.
              </p>
            </div>
            <Badge variant={canConfirm ? "default" : "secondary"}>
              {selected.size} / {count} selected
            </Badge>
          </div>
        </Modal.Header>

        <Modal.Instructions>
          Select the cards you want to put on the bottom of your library.
        </Modal.Instructions>

        <div className="p-6 overflow-y-auto">
          <div className="flex flex-wrap gap-4 justify-center">
            {handCards.map((card) => {
              const isSelected = selected.has(card.id);
              return (
                <div
                  key={card.id}
                  onClick={() => toggleCard(card.id)}
                  onMouseEnter={(e) => onMouseEnter(card, e)}
                  onMouseLeave={onMouseLeave}
                  className={cn(
                    "shrink-0 cursor-pointer transition-all rounded-lg",
                    isSelected
                      ? cn(CARD_RING.selected, "scale-105")
                      : "hover:ring-1 hover:ring-primary/50 hover:scale-[1.02]",
                  )}
                >
                  <Card card={card} className={MULLIGAN_CARD_SIZE} />
                </div>
              );
            })}
          </div>
        </div>

        <div className={MODAL_FOOTER_BETWEEN}>
          <span className="text-xs text-muted-foreground">
            These cards will go to the bottom of your library.
          </span>
          <Button
            size="sm"
            disabled={!canConfirm}
            onClick={handleConfirm}
            className="min-w-[100px]"
          >
            Confirm ({selected.size}/{count})
          </Button>
        </div>
      </div>

      {hoveredCard && (
        <CardPreview
          card={hoveredCard}
          mouseX={mousePos.x}
          mouseY={mousePos.y}
        />
      )}
    </Modal>
  );
}

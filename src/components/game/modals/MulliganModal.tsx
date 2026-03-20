import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Modal } from "./Modal";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { useHoverPreview } from "@/hooks/useHoverPreview";
import { MODAL_FOOTER_BETWEEN, MULLIGAN_CARD_SIZE } from "../game.styles";
import type { Card as CardType } from "@/types/openmagic";

interface MulliganModalProps {
  handCards: CardType[];
  mulliganCount: number;
  onKeep: () => void;
  onMulligan: () => void;
  isWaitingForResponse: boolean;
}

export function MulliganModal({
  handCards,
  mulliganCount,
  onKeep,
  onMulligan,
  isWaitingForResponse,
}: MulliganModalProps) {
  const putBackCount = mulliganCount;
  const { hoveredCard, mousePos, onMouseEnter, onMouseLeave } = useHoverPreview();

  return (
    <Modal maxWidth="max-w-[80vw]" maxHeight="max-h-[85vh]" className="w-fit">
      <Modal.Header>
        <div className="flex items-center justify-between">
          <div>
            <h2 className="font-semibold text-base">Opening Hand</h2>
            <p className="text-xs text-muted-foreground">
              {mulliganCount === 0
                ? "This is your opening hand."
                : `Mulligan ${mulliganCount} — you will put ${putBackCount} card${putBackCount !== 1 ? "s" : ""} back if you keep.`}
            </p>
          </div>
          <Badge variant="secondary">
            {handCards.length} cards
          </Badge>
        </div>
      </Modal.Header>

      <Modal.Instructions>
        Keep this hand or mulligan for a new one?
      </Modal.Instructions>

      <div className="p-6 overflow-y-auto">
        <div className="flex flex-wrap gap-4 justify-center">
          {handCards.map((card) => (
            <div
              key={card.id}
              className="shrink-0"
              onMouseEnter={(e) => onMouseEnter(card, e)}
              onMouseLeave={onMouseLeave}
            >
              <Card card={card} className={MULLIGAN_CARD_SIZE} />
            </div>
          ))}
        </div>
      </div>

      <div className={MODAL_FOOTER_BETWEEN}>
        <span className="text-xs text-muted-foreground">
          {mulliganCount === 0
            ? "First look — no penalty to keep."
            : `Keeping means putting ${putBackCount} card${putBackCount !== 1 ? "s" : ""} on the bottom of your library.`}
        </span>
        <div className="flex gap-2">
          <Button
            size="sm"
            variant="destructive"
            onClick={onMulligan}
            disabled={isWaitingForResponse}
            className="min-w-[100px]"
          >
            Mulligan
          </Button>
          <Button
            size="sm"
            onClick={onKeep}
            disabled={isWaitingForResponse}
            className="min-w-[100px]"
          >
            Keep Hand
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

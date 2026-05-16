import { Modal } from "../modals/Modal";
import { Button } from "@/components/ui/button";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { MODAL_CARD_IMAGE } from "../game.styles";
import { useModalKeyboard } from "@/hooks/useModalKeyboard";
import type { DeckCard } from "@/types/manabrew";

interface KickerModalProps {
  kickerCost: string;
  sourceCard?: DeckCard;
  onDecide: (kicked: boolean) => void;
}

export function KickerModal({ kickerCost, sourceCard, onDecide }: KickerModalProps) {
  useModalKeyboard({ onSpace: () => onDecide(true) }, [onDecide]);
  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <Modal.Header>
        <h2 className="font-semibold text-base">Pay Kicker?</h2>
      </Modal.Header>
      <div className="px-4 py-4 flex gap-3">
        {sourceCard && <CardImageThumbnail card={sourceCard} className={MODAL_CARD_IMAGE} />}
        <p className="text-sm text-muted-foreground self-center">
          Pay additional kicker cost: <ManaSymbols cost={kickerCost} size="lg" />
        </p>
      </div>
      <Modal.Footer>
        <Button variant="outline" onClick={() => onDecide(false)}>
          No
        </Button>
        <Button onClick={() => onDecide(true)}>Pay Kicker</Button>
      </Modal.Footer>
    </Modal>
  );
}

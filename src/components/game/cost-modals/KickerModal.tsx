import { Modal } from "../modals/Modal";
import { Button } from "@/components/ui/button";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { MODAL_CARD_IMAGE } from "../game.styles";

interface KickerModalProps {
  kickerCost: string;
  sourceCardName?: string;
  onDecide: (kicked: boolean) => void;
}

export function KickerModal({ kickerCost, sourceCardName, onDecide }: KickerModalProps) {
  const { data: imageUrl } = useCardImage(sourceCardName ?? "");
  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <Modal.Header>
        <h2 className="font-semibold text-base">Pay Kicker?</h2>
      </Modal.Header>
      <div className="px-4 py-4 flex gap-3">
        {imageUrl && (
          <CardImageThumbnail
            imageUrl={imageUrl}
            cardName={sourceCardName ?? "Spell"}
            className={MODAL_CARD_IMAGE}
          />
        )}
        <p className="text-sm text-muted-foreground self-center">
          Pay additional kicker cost: <ManaSymbols cost={kickerCost} size="lg" />
        </p>
      </div>
      <Modal.Footer>
        <Button variant="outline" onClick={() => onDecide(false)}>No</Button>
        <Button onClick={() => onDecide(true)}>Pay Kicker</Button>
      </Modal.Footer>
    </Modal>
  );
}
